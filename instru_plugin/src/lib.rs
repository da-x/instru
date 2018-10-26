#![feature(plugin_registrar, quote, rustc_private, custom_attribute)]

extern crate rustc_plugin;
extern crate syntax;

use rustc_plugin::registry::Registry;

use syntax::ast::*;
use syntax::source_map::{Span};
use syntax::ext::base::{ExtCtxt};
use syntax::fold::Folder;
use syntax::ptr::P;
use syntax::ext::base::{WholeCrateTransformer, WholeCrateTransformation};

struct Trans {
}

struct PosWrapper<'a, 'b: 'a> {
    _cx: &'a mut ExtCtxt<'b>,
    span: Span,
}

impl<'a, 'b> Folder for PosWrapper<'a, 'b> {
    fn fold_expr(&mut self, e: P<Expr>) -> P<Expr> {
        e.map(|e| {
            let mut e = syntax::fold::noop_fold_expr(e, self);
            e.span = self.span;
            e
        })
    }

    fn fold_mac(&mut self, mac: Mac) -> Mac {
        mac
    }
}

struct CodeWrapper<'a, 'b: 'a> {
    cx: &'a mut ExtCtxt<'b>
}


impl<'a, 'b> Folder for CodeWrapper<'a, 'b> {
    fn fold_crate(&mut self, b: Crate) -> Crate {
        let Crate { mut module, attrs, span } = syntax::fold::noop_fold_crate(b, self);

        match quote_item!(self.cx, extern crate instru;) {
            Some(item) => {
                module.items.insert(0, item);
            },
            None => {},
        }

        Crate {
            module,
            attrs,
            span
        }
    }

    fn fold_block(&mut self, b: P<Block>) -> P<Block> {
        let block = syntax::fold::noop_fold_block(b, self).into_inner();
        let Block {id, stmts, rules, span, recovered} = block;

        let block_instru_stmt = quote_stmt!(self.cx,
            let ___instru_v = ::instru::Wrapper::new(
                ::instru::Class::Block, function!(), module_path!(), file!(), line!())
        ).unwrap();

        let mut new_stmts = Vec::new();
        let mut stmt_idx = 0;

        if stmt_idx == 0 {
            let mut pos_wrapper = PosWrapper { _cx: self.cx, span: span };
            for stmt in pos_wrapper.fold_stmt(block_instru_stmt) {
                new_stmts.push(stmt);
            }
        }

        for stmt in stmts.into_iter() {
            stmt_idx += 1;

            let instru_stmt = quote_stmt!(self.cx,
                ::instru::statement(function!(), module_path!(), file!(), line!())
            ).unwrap();

            let mut pos_wrapper = PosWrapper { _cx: self.cx, span: stmt.span };
            for stmt in pos_wrapper.fold_stmt(instru_stmt) {
                new_stmts.push(stmt);
            }

            new_stmts.push(stmt);
        }

        P(Block {
            id,
            stmts: new_stmts,
            rules,
            span,
            recovered,
        })
    }

    fn fold_mac(&mut self, mac: Mac) -> Mac {
        mac
    }
}

impl WholeCrateTransformer for Trans {
    fn transform_before_expansion(&self, cx: &mut ExtCtxt, krate: Crate) -> Crate {
        let mut f = CodeWrapper { cx };
        f.fold_crate(krate)
    }
}

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    // println!("rustc plugin registrar called");
    reg.register_whole_crate_transformation(WholeCrateTransformation {
        cb: Box::new(Trans { })
    });
}

