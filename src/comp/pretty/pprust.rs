import std::_vec;
import std::_str;
import std::io;
import std::option;
import driver::session::session;
import front::ast;
import front::lexer;
import util::common;
import pp::end; import pp::wrd; import pp::space; import pp::line;

const uint indent_unit = 4u;
const uint default_columns = 78u;

type ps = @rec(pp::ps s,
               option::t[vec[lexer::cmnt]] comments,
               mutable uint cur_cmnt);

fn print_file(session sess, ast::_mod _mod, str filename, io::writer out) {
    auto cmnts = lexer::gather_comments(sess, filename);
    auto s = @rec(s=pp::mkstate(out, default_columns),
                  comments=option::some[vec[lexer::cmnt]](cmnts),
                  mutable cur_cmnt=0u);
    print_mod(s, _mod);
}

fn ty_to_str(&@ast::ty ty) -> str {
    auto writer = io::string_writer();
    auto s = @rec(s=pp::mkstate(writer.get_writer(), 0u),
                  comments=option::none[vec[lexer::cmnt]],
                  mutable cur_cmnt=0u);
    print_type(s, ty);
    ret writer.get_str();
}

fn block_to_str(&ast::block blk) -> str {
    auto writer = io::string_writer();
    auto s = @rec(s=pp::mkstate(writer.get_writer(), 78u),
                  comments=option::none[vec[lexer::cmnt]],
                  mutable cur_cmnt=0u);
    print_block(s, blk);
    ret writer.get_str();
}

fn pat_to_str(&@ast::pat p) -> str {
    auto writer = io::string_writer();
    auto s = @rec(s=pp::mkstate(writer.get_writer(), 78u),
                  comments=option::none[vec[lexer::cmnt]],
                  mutable cur_cmnt=0u);
    print_pat(s, p);
    ret writer.get_str();
}

fn hbox(ps s) {
    pp::hbox(s.s, indent_unit);
}
fn wrd1(ps s, str word) {
    wrd(s.s, word);
    space(s.s);
}
fn popen(ps s) {
    wrd(s.s, "(");
    pp::abox(s.s);
}
fn popen_h(ps s) {
    wrd(s.s, "(");
    pp::hbox(s.s, 0u);
}
fn pclose(ps s) {
    end(s.s);
    wrd(s.s, ")");
}
fn bopen(ps s) {
    wrd(s.s, "{");
    pp::vbox(s.s, indent_unit);
    line(s.s);
}
fn bclose(ps s) {
    end(s.s);
    pp::cwrd(s.s, "}");
}
fn bclose_c(ps s, common::span span) {
    maybe_print_comment(s, span.hi);
    bclose(s);
}
fn commasep[IN](ps s, vec[IN] elts, fn(ps, &IN) op) {
    auto first = true;
    for (IN elt in elts) {
        if (first) {first = false;}
        else {wrd1(s, ",");}
        op(s, elt);
    }
}
fn commasep_cmnt[IN](ps s, vec[IN] elts, fn(ps, &IN) op,
                            fn(&IN) -> common::span get_span) {
    auto len = _vec::len[IN](elts);
    auto i = 0u;
    for (IN elt in elts) {
        op(s, elt);
        i += 1u;
        if (i < len) {
            wrd(s.s, ",");
            if (!maybe_print_line_comment(s, get_span(elt))) {space(s.s);}
        }
    }
}
fn commasep_exprs(ps s, vec[@ast::expr] exprs) {
    fn expr_span(&@ast::expr expr) -> common::span {ret expr.span;}
    auto f = print_expr;
    auto gs = expr_span;
    commasep_cmnt[@ast::expr](s, exprs, f, gs);
}

fn print_mod(ps s, ast::_mod _mod) {
    for (@ast::view_item vitem in _mod.view_items) {
        print_view_item(s, vitem);
    }
    line(s.s);
    for (@ast::item item in _mod.items) {print_item(s, item);}
    print_remaining_comments(s);
}

fn print_type(ps s, &@ast::ty ty) {
    maybe_print_comment(s, ty.span.lo);
    hbox(s);
    alt (ty.node) {
        case (ast::ty_nil) {wrd(s.s, "()");}
        case (ast::ty_bool) {wrd(s.s, "bool");}
        case (ast::ty_int) {wrd(s.s, "int");}
        case (ast::ty_uint) {wrd(s.s, "uint");}
        case (ast::ty_float) {wrd(s.s, "float");}
        case (ast::ty_machine(?tm)) {wrd(s.s, common::ty_mach_to_str(tm));}
        case (ast::ty_char) {wrd(s.s, "char");}
        case (ast::ty_str) {wrd(s.s, "str");}
        case (ast::ty_box(?mt)) {wrd(s.s, "@"); print_mt(s, mt);}
        case (ast::ty_vec(?mt)) {
            wrd(s.s, "vec["); print_mt(s, mt); wrd(s.s, "]");
        }
        case (ast::ty_port(?t)) {
            wrd(s.s, "port["); print_type(s, t); wrd(s.s, "]");
        }
        case (ast::ty_chan(?t)) {
            wrd(s.s, "chan["); print_type(s, t); wrd(s.s, "]");
        }
        case (ast::ty_type) {wrd(s.s, "type");}
        case (ast::ty_tup(?elts)) {
            wrd(s.s, "tup");
            popen(s);
            auto f = print_mt;
            commasep[ast::mt](s, elts, f);
            pclose(s);
        }
        case (ast::ty_rec(?fields)) {
            wrd(s.s, "rec");
            popen(s);
            fn print_field(ps s, &ast::ty_field f) {
                hbox(s);
                print_mt(s, f.mt);
                space(s.s);
                wrd(s.s, f.ident);
                end(s.s);
            }
            fn get_span(&ast::ty_field f) -> common::span {
              // Try to reconstruct the span for this field
              auto sp = f.mt.ty.span;
              auto hi = sp.hi + _str::char_len(f.ident) + 1u;
              ret rec(hi=hi with sp);
            }
            auto f = print_field;
            auto gs = get_span;
            commasep_cmnt[ast::ty_field](s, fields, f, gs);
            pclose(s);
        }
        case (ast::ty_obj(?methods)) {
            wrd1(s, "obj");
            bopen(s);
            for (ast::ty_method m in methods) {
                hbox(s);
                print_ty_fn(s, m.proto, option::some[str](m.ident),
                            m.inputs, m.output);
                wrd(s.s, ";");
                end(s.s);
                line(s.s);
            }
            bclose_c(s, ty.span);
        }
        case (ast::ty_fn(?proto,?inputs,?output)) {
            print_ty_fn(s, proto, option::none[str], inputs, output);
        }
        case (ast::ty_path(?path,_)) {
            print_path(s, path);
        }
    }
    end(s.s);
}

fn print_item(ps s, @ast::item item) {
    maybe_print_comment(s, item.span.lo);
    hbox(s);
    alt (item.node) {
        case (ast::item_const(?id, ?ty, ?expr, _, _)) {
            wrd1(s, "const");
            print_type(s, ty);
            space(s.s);
            wrd1(s, id);
            wrd1(s, "=");
            print_expr(s, expr);
            wrd(s.s, ";");
        }
        case (ast::item_fn(?name,?_fn,?typarams,_,_)) {
            print_fn(s, _fn.decl, name, typarams);
            space(s.s);
            print_block(s, _fn.body);
        }
        case (ast::item_mod(?id,?_mod,_)) {
            wrd1(s, "mod");
            wrd1(s, id);
            bopen(s);
            for (@ast::item itm in _mod.items) {print_item(s, itm);}
            bclose_c(s, item.span);
        }
        case (ast::item_native_mod(?id,?nmod,_)) {
            wrd1(s, "native");
            alt (nmod.abi) {
                case (ast::native_abi_rust) {wrd1(s, "\"rust\"");}
                case (ast::native_abi_cdecl) {wrd1(s, "\"cdecl\"");}
                case (ast::native_abi_rust_intrinsic) {
                    wrd1(s, "\"rust-intrinstic\"");
                }
            }
            wrd1(s, "mod");
            wrd1(s, id);
            bopen(s);
            for (@ast::native_item item in nmod.items) {
                hbox(s);
                maybe_print_comment(s, item.span.lo);
                alt (item.node) {
                    case (ast::native_item_ty(?id,_)) {
                        wrd1(s, "type");
                        wrd(s.s, id);
                    }
                    case (ast::native_item_fn(?id,?lname,?decl,
                                             ?typarams,_,_)) {
                        print_fn(s, decl, id, typarams);
                        alt (lname) {
                            case (option::none[str]) {}
                            case (option::some[str](?ss)) {
                                print_string(s, ss);
                            }
                        }
                    }
                }
                wrd(s.s, ";");
                end(s.s);
            }
            bclose_c(s, item.span);
        }
        case (ast::item_ty(?id,?ty,?params,_,_)) {
            wrd1(s, "type");
            wrd(s.s, id);
            print_type_params(s, params);
            space(s.s);
            wrd1(s, "=");
            print_type(s, ty);
            wrd(s.s, ";");
        }
        case (ast::item_tag(?id,?variants,?params,_,_)) {
            wrd1(s, "tag");
            wrd(s.s, id);
            print_type_params(s, params);
            space(s.s);
            bopen(s);
            for (ast::variant v in variants) {
                maybe_print_comment(s, v.span.lo);
                wrd(s.s, v.node.name);
                if (_vec::len[ast::variant_arg](v.node.args) > 0u) {
                    popen(s);
                    fn print_variant_arg(ps s, &ast::variant_arg arg) {
                        print_type(s, arg.ty);
                    }
                    auto f = print_variant_arg;
                    commasep[ast::variant_arg](s, v.node.args, f);
                    pclose(s);
                }
                wrd(s.s, ";");
                if (!maybe_print_line_comment(s, v.span)) {line(s.s);}
            }
            bclose_c(s, item.span);
        }
        case (ast::item_obj(?id,?_obj,?params,_,_)) {
            wrd1(s, "obj");
            wrd(s.s, id);
            print_type_params(s, params);
            popen(s);
            fn print_field(ps s, &ast::obj_field field) {
                hbox(s);
                print_type(s, field.ty);
                space(s.s);
                wrd(s.s, field.ident);
                end(s.s);
            }
            fn get_span(&ast::obj_field f) -> common::span {ret f.ty.span;}
            auto f = print_field;
            auto gs = get_span;
            commasep_cmnt[ast::obj_field](s, _obj.fields, f, gs);
            pclose(s);
            space(s.s);
            bopen(s);
            for (@ast::method meth in _obj.methods) {
                hbox(s);
                let vec[ast::ty_param] typarams = vec();
                maybe_print_comment(s, meth.span.lo);
                print_fn(s, meth.node.meth.decl, meth.node.ident, typarams);
                space(s.s);
                print_block(s, meth.node.meth.body);
                end(s.s);
                line(s.s);
            }
            alt (_obj.dtor) {
                case (option::some[@ast::method](?dtor)) {
                    hbox(s);
                    wrd1(s, "close");
                    print_block(s, dtor.node.meth.body);
                    end(s.s);
                    line(s.s);
                }
                case (_) {}
            }
            bclose_c(s, item.span);
        }
    }
    end(s.s);
    line(s.s);
    line(s.s);
}

fn print_block(ps s, ast::block blk) {
    maybe_print_comment(s, blk.span.lo);
    bopen(s);
    for (@ast::stmt st in blk.node.stmts) {
        maybe_print_comment(s, st.span.lo);
        alt (st.node) {
          case (ast::stmt_decl(?decl,_)) {print_decl(s, decl);}
          case (ast::stmt_expr(?expr,_)) {print_expr(s, expr);}
        }
        if (front::parser::stmt_ends_with_semi(st)) {wrd(s.s, ";");}
        if (!maybe_print_line_comment(s, st.span)) {line(s.s);}
    }
    alt (blk.node.expr) {
        case (option::some[@ast::expr](?expr)) {
            print_expr(s, expr);
            if (!maybe_print_line_comment(s, expr.span)) {line(s.s);}
        }
        case (_) {}
    }
    bclose_c(s, blk.span);
}

fn print_literal(ps s, @ast::lit lit) {
    maybe_print_comment(s, lit.span.lo);
    alt (lit.node) {
        case (ast::lit_str(?st)) {print_string(s, st);}
        case (ast::lit_char(?ch)) {
            wrd(s.s, "'" + escape_str(_str::from_bytes(vec(ch as u8)), '\'')
                + "'");
        }
        case (ast::lit_int(?val)) {
            wrd(s.s, common::istr(val));
        }
        case (ast::lit_uint(?val)) { // FIXME clipping? uistr?
            wrd(s.s, common::istr(val as int) + "u");
        }
        case (ast::lit_float(?fstr)) {
            wrd(s.s, fstr);
        }
        case (ast::lit_mach_int(?mach,?val)) {
            wrd(s.s, common::istr(val as int));
            wrd(s.s, common::ty_mach_to_str(mach));
        }
        case (ast::lit_mach_float(?mach,?val)) {
            // val is already a str
            wrd(s.s, val);
            wrd(s.s, common::ty_mach_to_str(mach));
        }
        case (ast::lit_nil) {wrd(s.s, "()");}
        case (ast::lit_bool(?val)) {
            if (val) {wrd(s.s, "true");} else {wrd(s.s, "false");}
        }
    }
}

fn print_expr(ps s, &@ast::expr expr) {
    maybe_print_comment(s, expr.span.lo);
    hbox(s);
    alt (expr.node) {
        case (ast::expr_vec(?exprs,?mut,_)) {
            if (mut == ast::mut) {
                wrd1(s, "mutable");
            }
            wrd(s.s, "vec");
            popen(s);
            commasep_exprs(s, exprs);
            pclose(s);
        }
        case (ast::expr_tup(?exprs,_)) {
            fn printElt(ps s, &ast::elt elt) {
                hbox(s);
                if (elt.mut == ast::mut) {wrd1(s, "mutable");}
                print_expr(s, elt.expr);
                end(s.s);
            }
            fn get_span(&ast::elt elt) -> common::span {ret elt.expr.span;}
            wrd(s.s, "tup");
            popen(s);
            auto f = printElt;
            auto gs = get_span;
            commasep_cmnt[ast::elt](s, exprs, f, gs);
            pclose(s);
        }
        case (ast::expr_rec(?fields,?wth,_)) {
            fn print_field(ps s, &ast::field field) {
                hbox(s);
                if (field.mut == ast::mut) {wrd1(s, "mutable");}
                wrd(s.s, field.ident);
                wrd(s.s, "=");
                print_expr(s, field.expr);
                end(s.s);
            }
            fn get_span(&ast::field field) -> common::span {
                ret field.expr.span;
            }
            wrd(s.s, "rec");
            popen(s);
            auto f = print_field;
            auto gs = get_span;
            commasep_cmnt[ast::field](s, fields, f, gs);
            alt (wth) {
                case (option::some[@ast::expr](?expr)) {
                    if (_vec::len[ast::field](fields) > 0u) {space(s.s);}
                    hbox(s);
                    wrd1(s, "with");
                    print_expr(s, expr);
                    end(s.s);
                }
                case (_) {}
            }
            pclose(s);
        }
        case (ast::expr_call(?func,?args,_)) {
            print_expr(s, func);
            popen(s);
            commasep_exprs(s, args);
            pclose(s);
        }
        case (ast::expr_self_method(?ident,_)) {
            wrd(s.s, "self.");
            print_ident(s, ident);
        }
        case (ast::expr_bind(?func,?args,_)) {
            fn print_opt(ps s, &option::t[@ast::expr] expr) {
                alt (expr) {
                    case (option::some[@ast::expr](?expr)) {
                        print_expr(s, expr);
                    }
                    case (_) {wrd(s.s, "_");}
                }
            }
            wrd1(s, "bind");
            print_expr(s, func);
            popen(s);
            auto f = print_opt;
            commasep[option::t[@ast::expr]](s, args, f);
            pclose(s);
        }
    case (ast::expr_spawn(_,_,?e,?es,_)) {
          wrd1(s, "spawn");
          print_expr(s, e);
          popen(s);
          commasep_exprs(s, es);
          pclose(s);
        }
        case (ast::expr_binary(?op,?lhs,?rhs,_)) {
            auto prec = operator_prec(op);
            print_maybe_parens(s, lhs, prec);
            space(s.s);
            wrd1(s, ast::binop_to_str(op));
            print_maybe_parens(s, rhs, prec + 1);
        }
        case (ast::expr_unary(?op,?expr,_)) {
            wrd(s.s, ast::unop_to_str(op));
            print_expr(s, expr);
        }
        case (ast::expr_lit(?lit,_)) {
            print_literal(s, lit);
        }
        case (ast::expr_cast(?expr,?ty,_)) {
            print_maybe_parens(s, expr, front::parser::as_prec);
            space(s.s);
            wrd1(s, "as");
            print_type(s, ty);
        }
        case (ast::expr_if(?test,?block,?elseopt,_)) {
            wrd1(s, "if");
            popen_h(s);
            print_expr(s, test);
            pclose(s);
            space(s.s);
            print_block(s, block);
            alt (elseopt) {
                case (option::some[@ast::expr](?_else)) {
                    space(s.s);
                    wrd1(s, "else");
                    print_expr(s, _else);
                }
                case (_) { /* fall through */ }
            }
        }
        case (ast::expr_while(?test,?block,_)) {
            wrd1(s, "while");
            popen_h(s);
            print_expr(s, test);
            pclose(s);
            space(s.s);
            print_block(s, block);
        }
        case (ast::expr_for(?decl,?expr,?block,_)) {
            wrd1(s, "for");
            popen_h(s);
            print_for_decl(s, decl);
            space(s.s);
            wrd1(s, "in");
            print_expr(s, expr);
            pclose(s);
            space(s.s);
            print_block(s, block);
        }
        case (ast::expr_for_each(?decl,?expr,?block,_)) {
            wrd1(s, "for each");
            popen_h(s);
            print_for_decl(s, decl);
            space(s.s);
            wrd1(s, "in");
            print_expr(s, expr);
            pclose(s);
            space(s.s);
            print_block(s, block);
        }
        case (ast::expr_do_while(?block,?expr,_)) {
            wrd1(s, "do");
            space(s.s);
            print_block(s, block);
            space(s.s);
            wrd1(s, "while");
            popen_h(s);
            print_expr(s, expr);
            pclose(s);
        }
        case (ast::expr_alt(?expr,?arms,_)) {
            wrd1(s, "alt");
            popen_h(s);
            print_expr(s, expr);
            pclose(s);
            space(s.s);
            bopen(s);
            for (ast::arm arm in arms) {
                hbox(s);
                wrd1(s, "case");
                popen_h(s);
                print_pat(s, arm.pat);
                pclose(s);
                space(s.s);
                print_block(s, arm.block);
                end(s.s);
                line(s.s);
            }
            bclose_c(s, expr.span);
        }
        case (ast::expr_block(?block,_)) {
            print_block(s, block);
        }
        case (ast::expr_assign(?lhs,?rhs,_)) {
            print_expr(s, lhs);
            space(s.s);
            wrd1(s, "=");
            print_expr(s, rhs);
        }
        case (ast::expr_assign_op(?op,?lhs,?rhs,_)) {
            print_expr(s, lhs);
            space(s.s);
            wrd(s.s, ast::binop_to_str(op));
            wrd1(s, "=");
            print_expr(s, rhs);
        }
        case (ast::expr_send(?lhs, ?rhs, _)) {
            print_expr(s, lhs);
            space(s.s);
            wrd1(s, "<|");
            print_expr(s, rhs);
        }
        case (ast::expr_recv(?lhs, ?rhs, _)) {
            print_expr(s, lhs);
            space(s.s);
            wrd1(s, "<-");
            print_expr(s, rhs);
        }
        case (ast::expr_field(?expr,?id,_)) {
            print_expr(s, expr);
            wrd(s.s, ".");
            wrd(s.s, id);
        }
        case (ast::expr_index(?expr,?index,_)) {
            print_expr(s, expr);
            wrd(s.s, ".");
            popen_h(s);
            print_expr(s, index);
            pclose(s);
        }
        case (ast::expr_path(?path,_)) {
            print_path(s, path);
        }
        case (ast::expr_fail(_)) {
            wrd(s.s, "fail");
        }
        case (ast::expr_break(_)) {
            wrd(s.s, "break");
        }
        case (ast::expr_cont(_)) {
            wrd(s.s, "cont");
        }
        case (ast::expr_ret(?result,_)) {
            wrd(s.s, "ret");
            alt (result) {
                case (option::some[@ast::expr](?expr)) {
                    space(s.s);
                    print_expr(s, expr);
                }
                case (_) {}
            }
        }
        case (ast::expr_put(?result,_)) {
            wrd(s.s, "put");
            alt (result) {
                case (option::some[@ast::expr](?expr)) {
                    space(s.s);
                    print_expr(s, expr);
                }
                case (_) {}
            }
        }
        case (ast::expr_be(?result,_)) {
            wrd1(s, "be");
            print_expr(s, result);
        }
        case (ast::expr_log(?lvl,?expr,_)) {
            alt (lvl) {
                case (1) {wrd1(s, "log");}
                case (0) {wrd1(s, "log_err");}
            }
            print_expr(s, expr);
        }
        case (ast::expr_check(?expr,_)) {
            wrd1(s, "check");
            popen_h(s);
            print_expr(s, expr);
            pclose(s);
        }
        case (ast::expr_assert(?expr,_)) {
            wrd1(s, "assert");
            popen_h(s);
            print_expr(s, expr);
            pclose(s);
        }
        case (ast::expr_ext(?path, ?args, ?body, _, _)) {
            wrd(s.s, "#");
            print_path(s, path);
            if (_vec::len[@ast::expr](args) > 0u) {
                popen(s);
                commasep_exprs(s, args);
                pclose(s);
            }
            // FIXME: extension 'body'
        }
        case (ast::expr_port(_)) {
            wrd(s.s, "port");
            popen_h(s);
            pclose(s);
        }
        case (ast::expr_chan(?expr, _)) {
            wrd(s.s, "chan");
            popen_h(s);
            print_expr(s, expr);
            pclose(s);
        }

        case (ast::expr_anon_obj(_,_,_,_)) {
            wrd(s.s, "obj");
            // TODO
        }
    }
    end(s.s);
}

fn print_decl(ps s, @ast::decl decl) {
    maybe_print_comment(s, decl.span.lo);
    hbox(s);
    alt (decl.node) {
        case (ast::decl_local(?loc)) {
            alt (loc.ty) {
                case (option::some[@ast::ty](?ty)) {
                    wrd1(s, "let");
                    print_type(s, ty);
                    space(s.s);
                }
                case (_) {
                    wrd1(s, "auto");
                }
            }
            wrd(s.s, loc.ident);
            alt (loc.init) {
                case (option::some[ast::initializer](?init)) {
                    space(s.s);
                    alt (init.op) {
                        case (ast::init_assign) {
                            wrd1(s, "=");
                        }
                        case (ast::init_recv) {
                            wrd1(s, "<-");
                        }
                    }
                    print_expr(s, init.expr);
                }
                case (_) {}
            }
        }
        case (ast::decl_item(?item)) {
            print_item(s, item);
        }
    }
    end(s.s);
}

fn print_ident(ps s, ast::ident ident) {
    wrd(s.s, ident);
}

fn print_for_decl(ps s, @ast::decl decl) {
    alt (decl.node) {
        case (ast::decl_local(?loc)) {
            print_type(s, option::get[@ast::ty](loc.ty));
            space(s.s);
            wrd(s.s, loc.ident);
        }
    }
}

fn print_path(ps s, ast::path path) {
    maybe_print_comment(s, path.span.lo);
    auto first = true;
    for (str id in path.node.idents) {
        if (first) {first = false;}
        else {wrd(s.s, "::");}
        wrd(s.s, id);
    }
    if (_vec::len[@ast::ty](path.node.types) > 0u) {
        wrd(s.s, "[");
        auto f = print_type;
        commasep[@ast::ty](s, path.node.types, f);
        wrd(s.s, "]");
    }
}

fn print_pat(ps s, &@ast::pat pat) {
    maybe_print_comment(s, pat.span.lo);
    alt (pat.node) {
        case (ast::pat_wild(_)) {wrd(s.s, "_");}
        case (ast::pat_bind(?id,_,_)) {wrd(s.s, "?" + id);}
        case (ast::pat_lit(?lit,_)) {print_literal(s, lit);}
        case (ast::pat_tag(?path,?args,_)) {
            print_path(s, path);
            if (_vec::len[@ast::pat](args) > 0u) {
                popen_h(s);
                auto f = print_pat;
                commasep[@ast::pat](s, args, f);
                pclose(s);
            }
        }
    }
}

fn print_fn(ps s, ast::fn_decl decl, str name,
                   vec[ast::ty_param] typarams) {
    alt (decl.purity) {
        case (ast::impure_fn) {
            wrd1(s, "fn");
        }
        case (_) {
            wrd1(s, "pred");
        }
    }
    wrd(s.s, name);
    print_type_params(s, typarams);
    popen(s);
    fn print_arg(ps s, &ast::arg x) {
        hbox(s);
        if (x.mode == ast::alias) {wrd(s.s, "&");}
        print_type(s, x.ty);
        space(s.s);
        wrd(s.s, x.ident);
        end(s.s);
    }
    auto f = print_arg;
    commasep[ast::arg](s, decl.inputs, f);
    pclose(s);
    maybe_print_comment(s, decl.output.span.lo);
    if (decl.output.node != ast::ty_nil) {
        space(s.s);
        hbox(s);
        wrd1(s, "->");
        print_type(s, decl.output);
        end(s.s);
    }
}

fn print_type_params(ps s, vec[ast::ty_param] params) {
    if (_vec::len[ast::ty_param](params) > 0u) {
        wrd(s.s, "[");
        fn printParam(ps s, &ast::ty_param param) {
            wrd(s.s, param);
        }
        auto f = printParam;
        commasep[ast::ty_param](s, params, f);
        wrd(s.s, "]");
    }
}

fn print_view_item(ps s, @ast::view_item item) {
    maybe_print_comment(s, item.span.lo);
    hbox(s);
    alt (item.node) {
        case (ast::view_item_use(?id,?mta,_,_)) {
            wrd1(s, "use");
            wrd(s.s, id);
            if (_vec::len[@ast::meta_item](mta) > 0u) {
                popen(s);
                fn print_meta(ps s, &@ast::meta_item item) {
                    hbox(s);
                    wrd1(s, item.node.name);
                    wrd1(s, "=");
                    print_string(s, item.node.value);
                    end(s.s);
                }
                auto f = print_meta;
                commasep[@ast::meta_item](s, mta, f);
                pclose(s);
            }
        }
        case (ast::view_item_import(?id,?ids,_)) {
            wrd1(s, "import");
            if (!_str::eq(id, ids.(_vec::len[str](ids)-1u))) {
                wrd1(s, id);
                wrd1(s, "=");
            }
            auto first = true;
            for (str elt in ids) {
                if (first) {first = false;}
                else {wrd(s.s, ":");}
                wrd(s.s, elt);
            }
        }
        case (ast::view_item_export(?id)) {
            wrd1(s, "export");
            wrd(s.s, id);
        }
    }
    end(s.s);
    wrd(s.s, ";");
    line(s.s);
}

// FIXME: The fact that this builds up the table anew for every call is
// not good. Eventually, table should be a const.
fn operator_prec(ast::binop op) -> int {
    for (front::parser::op_spec spec in front::parser::prec_table()) {
        if (spec.op == op) {ret spec.prec;}
    }
    fail;
}

fn print_maybe_parens(ps s, @ast::expr expr, int outer_prec) {
    auto add_them;
    alt (expr.node) {
        case (ast::expr_binary(?op,_,_,_)) {
            add_them = operator_prec(op) < outer_prec;
        }
        case (ast::expr_cast(_,_,_)) {
            add_them = front::parser::as_prec < outer_prec;
        }
        case (_) {
            add_them = false;
        }
    }
    if (add_them) {popen(s);}
    print_expr(s, expr);
    if (add_them) {pclose(s);}
}

fn escape_str(str st, char to_escape) -> str {
    let str out = "";
    auto len = _str::byte_len(st);
    auto i = 0u;
    while (i < len) {
        alt (st.(i) as char) {
            case ('\n') {out += "\\n";}
            case ('\t') {out += "\\t";}
            case ('\r') {out += "\\r";}
            case ('\\') {out += "\\\\";}
            case (?cur) {
                if (cur == to_escape) {out += "\\";}
                _str::push_byte(out, cur as u8);
            }
        }
        i += 1u;
    }
    ret out;
}

fn print_mt(ps s, &ast::mt mt) {
    alt (mt.mut) {
        case (ast::mut)       { wrd1(s, "mutable");  }
        case (ast::maybe_mut) { wrd1(s, "mutable?"); }
        case (ast::imm)       { /* nothing */        }
    }
    print_type(s, mt.ty);
}

fn print_string(ps s, str st) {
    wrd(s.s, "\""); wrd(s.s, escape_str(st, '"')); wrd(s.s, "\"");
}

fn print_ty_fn(ps s, ast::proto proto, option::t[str] id,
               vec[ast::ty_arg] inputs, @ast::ty output) {
    if (proto == ast::proto_fn) {wrd(s.s, "fn");}
    else {wrd(s.s, "iter");}
    alt (id) {
        case (option::some[str](?id)) {space(s.s); wrd(s.s, id);}
        case (_) {}
    }
    popen_h(s);
    fn print_arg(ps s, &ast::ty_arg input) {
        if (input.mode == ast::alias) {wrd(s.s, "&");}
        print_type(s, input.ty);
    }
    auto f = print_arg;
    commasep[ast::ty_arg](s, inputs, f);
    pclose(s);
    maybe_print_comment(s, output.span.lo);
    if (output.node != ast::ty_nil) {
        space(s.s);
        hbox(s);
        wrd1(s, "->");
        print_type(s, output);
        end(s.s);
    }
}

fn next_comment(ps s) -> option::t[lexer::cmnt] {
    alt (s.comments) {
        case (option::some[vec[lexer::cmnt]](?cmnts)) {
            if (s.cur_cmnt < _vec::len[lexer::cmnt](cmnts)) {
                ret option::some[lexer::cmnt](cmnts.(s.cur_cmnt));
            } else {ret option::none[lexer::cmnt];}
        }
        case (_) {ret option::none[lexer::cmnt];}
    }
}

fn maybe_print_comment(ps s, uint pos) {
    while (true) {
        alt (next_comment(s)) {
            case (option::some[lexer::cmnt](?cmnt)) {
                if (cmnt.pos < pos) {
                    print_comment(s, cmnt.val);
                    if (cmnt.space_after) {line(s.s);}
                    s.cur_cmnt += 1u;
                } else { break; }
            }
            case (_) {break;}
        }
    }
}

fn maybe_print_line_comment(ps s, common::span span) -> bool {
    alt (next_comment(s)) {
        case (option::some[lexer::cmnt](?cmnt)) {
            if (span.hi + 4u >= cmnt.pos) {
                wrd(s.s, " ");
                print_comment(s, cmnt.val);
                s.cur_cmnt += 1u;
                ret true;
            }
        }
        case (_) {}
    }
    ret false;
}

fn print_remaining_comments(ps s) {
    while (true) {
        alt (next_comment(s)) {
            case (option::some[lexer::cmnt](?cmnt)) {
                print_comment(s, cmnt.val);
                if (cmnt.space_after) {line(s.s);}
                s.cur_cmnt += 1u;
            }
            case (_) {break;}
        }
    }
}

fn print_comment(ps s, lexer::cmnt_ cmnt) {
    alt (cmnt) {
        case (lexer::cmnt_line(?val)) {
            wrd(s.s, "// " + val);
            pp::hardbreak(s.s);
        }
        case (lexer::cmnt_block(?lines)) {
            pp::abox(s.s);
            wrd(s.s, "/* ");
            pp::abox(s.s);
            auto first = true;
            for (str ln in lines) {
                if (first) {first = false;}
                else {pp::hardbreak(s.s);}
                wrd(s.s, ln);
            }
            end(s.s);
            wrd(s.s, "*/");
            end(s.s);
            line(s.s);
        }
    }
}
//
// Local Variables:
// mode: rust
// fill-column: 78;
// indent-tabs-mode: nil
// c-basic-offset: 4
// buffer-file-coding-system: utf-8-unix
// compile-command: "make -k -C $RBUILD 2>&1 | sed -e 's/\\/x\\//x:\\//g'";
// End:
//
