use super::cst::*;
use super::lexer::{LexMode, Lexer};
use super::token::{Span, Token, TokenKind};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq)]
#[error("{message} at line {line}:{col}", line = span.line, col = span.col)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl ParseError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }

    pub fn unexpected(expected: impl Into<String>, found: &TokenKind, span: Span) -> Self {
        Self {
            message: format!("Expected {}, found {:?}", expected.into(), found),
            span,
        }
    }
}

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current: Token,
    peeked: Option<Token>,
}

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let current = lexer.next_token();
        Self {
            lexer,
            current,
            peeked: None,
        }
    }

    pub fn current(&self) -> &Token {
        &self.current
    }

    pub fn peek(&mut self) -> &Token {
        if self.peeked.is_none() {
            self.peeked = Some(self.lexer.next_token());
        }
        self.peeked.as_ref().unwrap()
    }

    pub fn advance(&mut self) -> Token {
        if let Some(t) = self.peeked.take() {
            std::mem::replace(&mut self.current, t)
        } else {
            let next = self.lexer.next_token();
            std::mem::replace(&mut self.current, next)
        }
    }

    pub fn check(&self, kind: &TokenKind) -> bool {
        &self.current.kind == kind
    }

    pub fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub fn expect(&mut self, kind: &TokenKind) -> Result<Token, ParseError> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(ParseError {
                message: format!("Expected '{}', found '{}'", kind, self.current.kind),
                span: self.current.span,
            })
        }
    }

    pub fn skip_terminators(&mut self) {
        while self.check(&TokenKind::Newline) || self.check(&TokenKind::Semicolon) {
            self.advance();
        }
    }

    // ── Module & Top-level ──────────────────────────────────────────────────

    pub fn parse_module(&mut self) -> Result<Module, ParseError> {
        let start_span = self.current.span;
        let mut items = Vec::new();

        self.skip_terminators();
        while !self.check(&TokenKind::Eof) {
            let item = self.parse_item()?;
            items.push(item);
            self.skip_terminators();
        }

        let end_span = self.current.span;
        Ok(Module {
            items,
            span: start_span.merge(end_span),
        })
    }

    pub fn parse_item(&mut self) -> Result<Item, ParseError> {
        match &self.current.kind {
            TokenKind::Fn => {
                let decl = self.parse_fn_def()?;
                Ok(Item::Decl(Decl::Fn(decl)))
            }
            TokenKind::Struct => {
                let decl = self.parse_struct_def()?;
                Ok(Item::Decl(Decl::Struct(decl)))
            }
            TokenKind::Enum => {
                let decl = self.parse_enum_def()?;
                Ok(Item::Decl(Decl::Enum(decl)))
            }
            TokenKind::Protocol => {
                let decl = self.parse_protocol_def()?;
                Ok(Item::Decl(Decl::Protocol(decl)))
            }
            TokenKind::Impl => {
                let decl = self.parse_impl_def()?;
                Ok(Item::Decl(Decl::Impl(decl)))
            }
            TokenKind::Import => {
                let decl = self.parse_import_decl()?;
                Ok(Item::Decl(Decl::Import(decl)))
            }
            _ => {
                let stmt = self.parse_stmt()?;
                Ok(Item::Stmt(stmt))
            }
        }
    }

    // ── Declarations ────────────────────────────────────────────────────────

    pub fn parse_fn_def(&mut self) -> Result<FnDef, ParseError> {
        let fn_tok = self.expect(&TokenKind::Fn)?;
        let name = self.parse_ident_name()?;
        let type_params = self.parse_opt_type_params()?;

        self.expect(&TokenKind::LParen)?;
        self.lexer.push_mode(LexMode::ExprMl);
        let params = self.parse_param_list()?;
        self.expect(&TokenKind::RParen)?;
        self.lexer.pop_mode();

        let ret_type = if self.match_token(&TokenKind::Arrow) {
            Some(self.parse_type_ann()?)
        } else {
            None
        };

        if self.check(&TokenKind::Colon) {
            self.advance();
        }
        self.skip_terminators();

        let body = self.parse_block()?;
        let span = fn_tok.span.merge(body.span);

        Ok(FnDef {
            name,
            type_params,
            params,
            ret_type,
            body,
            span,
        })
    }

    pub fn parse_struct_def(&mut self) -> Result<StructDef, ParseError> {
        let kw = self.expect(&TokenKind::Struct)?;
        let name = self.parse_ident_name()?;
        let type_params = self.parse_opt_type_params()?;

        if self.check(&TokenKind::Colon) {
            self.advance();
        }
        self.skip_terminators();

        let _lbrace = self.expect(&TokenKind::LBrace)?;
        self.lexer.push_mode(LexMode::ExprMl);
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            self.skip_terminators();
            if self.check(&TokenKind::RBrace) {
                break;
            }
            let field_name = self.parse_ident_name()?;
            if self.check(&TokenKind::Colon) {
                self.advance();
            }
            let type_ann = self.parse_type_ann()?;
            let span = type_ann.span();
            fields.push(FieldDef {
                name: field_name,
                type_ann,
                span,
            });
            if self.check(&TokenKind::Comma) || self.check(&TokenKind::Semicolon) {
                self.advance();
            }
            self.skip_terminators();
        }
        let rbrace = self.expect(&TokenKind::RBrace)?;
        self.lexer.pop_mode();

        Ok(StructDef {
            name,
            type_params,
            fields,
            span: kw.span.merge(rbrace.span),
        })
    }

    pub fn parse_enum_def(&mut self) -> Result<EnumDef, ParseError> {
        let kw = self.expect(&TokenKind::Enum)?;
        let name = self.parse_ident_name()?;
        let type_params = self.parse_opt_type_params()?;

        if self.check(&TokenKind::Colon) {
            self.advance();
        }
        self.skip_terminators();

        let lbrace = self.expect(&TokenKind::LBrace)?;
        self.lexer.push_mode(LexMode::ExprMl);
        let mut variants = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            self.skip_terminators();
            if self.check(&TokenKind::RBrace) {
                break;
            }
            let v_name = self.parse_ident_name()?;
            let mut payload = Vec::new();
            if self.match_token(&TokenKind::LParen) {
                while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                    payload.push(self.parse_type_ann()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RParen)?;
            }
            variants.push(VariantDef {
                name: v_name,
                payload,
                span: lbrace.span,
            });
            if self.check(&TokenKind::Comma) || self.check(&TokenKind::Semicolon) {
                self.advance();
            }
            self.skip_terminators();
        }
        let rbrace = self.expect(&TokenKind::RBrace)?;
        self.lexer.pop_mode();

        Ok(EnumDef {
            name,
            type_params,
            variants,
            span: kw.span.merge(rbrace.span),
        })
    }

    pub fn parse_protocol_def(&mut self) -> Result<ProtocolDef, ParseError> {
        let kw = self.expect(&TokenKind::Protocol)?;
        let name = self.parse_ident_name()?;
        let type_params = self.parse_opt_type_params()?;

        if self.check(&TokenKind::Colon) {
            self.advance();
        }
        self.skip_terminators();

        self.expect(&TokenKind::LBrace)?;
        self.lexer.push_mode(LexMode::ExprMl);
        let mut properties = Vec::new();
        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            self.skip_terminators();
            if self.check(&TokenKind::RBrace) {
                break;
            }
            if self.check(&TokenKind::Fn) {
                let fn_tok = self.advance();
                let m_name = self.parse_ident_name()?;
                let m_tparams = self.parse_opt_type_params()?;
                self.expect(&TokenKind::LParen)?;
                let params = self.parse_param_list()?;
                self.expect(&TokenKind::RParen)?;
                let ret = if self.match_token(&TokenKind::Arrow) {
                    Some(self.parse_type_ann()?)
                } else {
                    None
                };
                methods.push(FnSig {
                    name: m_name,
                    type_params: m_tparams,
                    params,
                    ret_type: ret,
                    span: fn_tok.span,
                });
            } else {
                let prop_name = self.parse_ident_name()?;
                if self.check(&TokenKind::Colon) {
                    self.advance();
                }
                let type_ann = self.parse_type_ann()?;
                let span = type_ann.span();
                properties.push(FieldDef {
                    name: prop_name,
                    type_ann,
                    span,
                });
            }
            if self.check(&TokenKind::Comma) || self.check(&TokenKind::Semicolon) {
                self.advance();
            }
            self.skip_terminators();
        }
        let rbrace = self.expect(&TokenKind::RBrace)?;
        self.lexer.pop_mode();

        Ok(ProtocolDef {
            name,
            type_params,
            properties,
            methods,
            span: kw.span.merge(rbrace.span),
        })
    }

    pub fn parse_impl_def(&mut self) -> Result<ImplDef, ParseError> {
        let kw = self.expect(&TokenKind::Impl)?;
        let target_type = self.parse_type_ann()?;
        let for_protocol = if self.check(&TokenKind::Ident("for".to_string())) {
            self.advance();
            Some(self.parse_type_ann()?)
        } else {
            None
        };

        if self.check(&TokenKind::Colon) {
            self.advance();
        }
        self.skip_terminators();

        self.expect(&TokenKind::LBrace)?;
        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            self.skip_terminators();
            if self.check(&TokenKind::RBrace) {
                break;
            }
            methods.push(self.parse_fn_def()?);
            self.skip_terminators();
        }
        let rbrace = self.expect(&TokenKind::RBrace)?;

        Ok(ImplDef {
            target_type,
            for_protocol,
            methods,
            span: kw.span.merge(rbrace.span),
        })
    }

    pub fn parse_import_decl(&mut self) -> Result<ImportDef, ParseError> {
        let start_tok = self.expect(&TokenKind::Import)?;
        let mut end_span;
        let target = match &self.current.kind {
            TokenKind::String(s) => {
                let path = s.clone();
                end_span = self.current.span;
                self.advance();
                ImportTarget::Path(path)
            }
            TokenKind::Ident(_) => {
                end_span = self.current.span;
                let mut parts = vec![self.parse_ident_name()?];
                while self.match_token(&TokenKind::Dot) {
                    end_span = self.current.span;
                    parts.push(self.parse_ident_name()?);
                }
                if parts.len() == 1 {
                    ImportTarget::Ident(parts.pop().unwrap())
                } else {
                    ImportTarget::Dotted(parts)
                }
            }
            other => {
                return Err(ParseError {
                    message: format!("Expected module path or string after 'import', found '{}'", other),
                    span: self.current.span,
                });
            }
        };
        let span = start_tok.span.merge(end_span);
        if self.check(&TokenKind::Semicolon) {
            self.advance();
        }
        Ok(ImportDef { target, span })
    }

    // ── Statements ──────────────────────────────────────────────────────────

    pub fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        match &self.current.kind {
            TokenKind::Let => {
                let kw = self.advance();
                let name = self.parse_ident_name()?;
                let type_ann = if self.match_token(&TokenKind::Colon) {
                    Some(self.parse_type_ann()?)
                } else {
                    None
                };
                self.expect(&TokenKind::Eq)?;
                let init = self.parse_expr()?;
                let span = kw.span.merge(init.span());
                if self.check(&TokenKind::Semicolon) {
                    self.advance();
                }
                Ok(Stmt::Let {
                    name,
                    type_ann,
                    init,
                    span,
                })
            }
            TokenKind::Const => {
                let kw = self.advance();
                let name = self.parse_ident_name()?;
                let type_ann = if self.match_token(&TokenKind::Colon) {
                    Some(self.parse_type_ann()?)
                } else {
                    None
                };
                self.expect(&TokenKind::Eq)?;
                let init = self.parse_expr()?;
                let span = kw.span.merge(init.span());
                if self.check(&TokenKind::Semicolon) {
                    self.advance();
                }
                Ok(Stmt::Const {
                    name,
                    type_ann,
                    init,
                    span,
                })
            }
            TokenKind::While => {
                let kw = self.advance();
                let cond = self.parse_expr()?;
                if self.check(&TokenKind::Colon) {
                    self.advance();
                }
                self.skip_terminators();
                let body = self.parse_block()?;
                let span = kw.span.merge(body.span);
                Ok(Stmt::While { cond, body, span })
            }
            TokenKind::Loop => {
                let kw = self.advance();
                if self.check(&TokenKind::Colon) {
                    self.advance();
                }
                self.skip_terminators();
                let body = self.parse_block()?;
                let span = kw.span.merge(body.span);
                Ok(Stmt::Loop { body, span })
            }
            TokenKind::Return => {
                let kw = self.advance();
                let val = if self.check(&TokenKind::Semicolon)
                    || self.check(&TokenKind::Newline)
                    || self.check(&TokenKind::RBrace)
                    || self.check(&TokenKind::Eof)
                {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                let span = if let Some(ref v) = val {
                    kw.span.merge(v.span())
                } else {
                    kw.span
                };
                if self.check(&TokenKind::Semicolon) {
                    self.advance();
                }
                Ok(Stmt::Return { val, span })
            }
            TokenKind::Break => {
                let tok = self.advance();
                if self.check(&TokenKind::Semicolon) {
                    self.advance();
                }
                Ok(Stmt::Break(tok.span))
            }
            TokenKind::Continue => {
                let tok = self.advance();
                if self.check(&TokenKind::Semicolon) {
                    self.advance();
                }
                Ok(Stmt::Continue(tok.span))
            }
            _ => {
                let expr = self.parse_expr()?;
                if self.match_token(&TokenKind::Eq) {
                    let val = self.parse_expr()?;
                    let span = expr.span().merge(val.span());
                    if self.check(&TokenKind::Semicolon) {
                        self.advance();
                    }
                    Ok(Stmt::Assign {
                        target: expr,
                        val,
                        span,
                    })
                } else if let Some(op) = self.match_aug_assign_op() {
                    let val = self.parse_expr()?;
                    let span = expr.span().merge(val.span());
                    if self.check(&TokenKind::Semicolon) {
                        self.advance();
                    }
                    Ok(Stmt::AugAssign {
                        target: expr,
                        op,
                        val,
                        span,
                    })
                } else {
                    if self.check(&TokenKind::Semicolon) {
                        self.advance();
                    }
                    Ok(Stmt::Expr(expr))
                }
            }
        }
    }

    fn match_aug_assign_op(&mut self) -> Option<BinOp> {
        let op = match &self.current.kind {
            TokenKind::PlusEq => Some(BinOp::Add),
            TokenKind::MinusEq => Some(BinOp::Sub),
            TokenKind::StarEq => Some(BinOp::Mul),
            TokenKind::SlashEq => Some(BinOp::Div),
            TokenKind::RemEq => Some(BinOp::Rem),
            _ => None,
        };
        if op.is_some() {
            self.advance();
        }
        op
    }

    // ── Block ───────────────────────────────────────────────────────────────

    pub fn parse_block(&mut self) -> Result<Block, ParseError> {
        let lbrace = self.expect(&TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        let mut tail_expr = None;

        self.skip_terminators();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let stmt = self.parse_stmt()?;
            self.skip_terminators();

            // Check if last stmt can be tail expression
            if self.check(&TokenKind::RBrace) {
                if let Stmt::Expr(e) = stmt {
                    tail_expr = Some(Box::new(e));
                    break;
                }
            }
            stmts.push(stmt);
            self.skip_terminators();
        }
        let rbrace = self.expect(&TokenKind::RBrace)?;

        Ok(Block {
            stmts,
            tail_expr,
            span: lbrace.span.merge(rbrace.span),
        })
    }

    // ── Expressions (Pratt Precedence) ──────────────────────────────────────

    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_expr_prec(0)
    }

    fn parse_expr_prec(&mut self, min_prec: u8) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary_or_primary()?;

        while let Some((op, prec, r_assoc)) = self.current_infix_op() {
            if prec < min_prec {
                break;
            }
            self.advance();
            let next_prec = if r_assoc { prec } else { prec + 1 };
            let right = self.parse_expr_prec(next_prec)?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn current_infix_op(&self) -> Option<(BinOp, u8, bool)> {
        match &self.current.kind {
            TokenKind::Or => Some((BinOp::Or, 1, false)),
            TokenKind::And => Some((BinOp::And, 2, false)),
            TokenKind::EqEq => Some((BinOp::Eq, 3, false)),
            TokenKind::NotEq => Some((BinOp::NotEq, 3, false)),
            TokenKind::Lt => Some((BinOp::Lt, 4, false)),
            TokenKind::LtEq => Some((BinOp::LtEq, 4, false)),
            TokenKind::Gt => Some((BinOp::Gt, 4, false)),
            TokenKind::GtEq => Some((BinOp::GtEq, 4, false)),
            TokenKind::Plus => Some((BinOp::Add, 5, false)),
            TokenKind::Minus => Some((BinOp::Sub, 5, false)),
            TokenKind::Star => Some((BinOp::Mul, 6, false)),
            TokenKind::Slash => Some((BinOp::Div, 6, false)),
            TokenKind::Rem => Some((BinOp::Rem, 6, false)),
            _ => None,
        }
    }

    fn parse_unary_or_primary(&mut self) -> Result<Expr, ParseError> {
        match &self.current.kind {
            TokenKind::Minus => {
                let op_tok = self.advance();
                let operand = self.parse_unary_or_primary()?;
                let span = op_tok.span.merge(operand.span());
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                    span,
                })
            }
            TokenKind::Not => {
                let op_tok = self.advance();
                let operand = self.parse_unary_or_primary()?;
                let span = op_tok.span.merge(operand.span());
                Ok(Expr::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                    span,
                })
            }
            _ => self.parse_postfix_expr(),
        }
    }

    fn parse_postfix_expr(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.check(&TokenKind::LParen) {
                // Call
                self.advance();
                self.lexer.push_mode(LexMode::ExprMl);
                let mut args = Vec::new();
                while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                    args.push(self.parse_expr()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                let rparen = self.expect(&TokenKind::RParen)?;
                self.lexer.pop_mode();
                let span = expr.span().merge(rparen.span);
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                    span,
                };
            } else if self.check(&TokenKind::Dot) {
                // Field access
                self.advance();
                let field_tok = self.expect_ident()?;
                let span = expr.span().merge(field_tok.span);
                expr = Expr::FieldAccess {
                    base: Box::new(expr),
                    field: field_tok.text,
                    span,
                };
            } else if self.check(&TokenKind::LBracket) {
                // Index
                self.advance();
                self.lexer.push_mode(LexMode::ExprMl);
                let index = self.parse_expr()?;
                let rbracket = self.expect(&TokenKind::RBracket)?;
                self.lexer.pop_mode();
                let span = expr.span().merge(rbracket.span);
                expr = Expr::Index {
                    base: Box::new(expr),
                    index: Box::new(index),
                    span,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match &self.current.kind {
            TokenKind::Int(i) => {
                let val = *i;
                let tok = self.advance();
                Ok(Expr::Literal(LitVal::Int(val), tok.span))
            }
            TokenKind::Float(bits) => {
                let val = f64::from_bits(*bits);
                let tok = self.advance();
                Ok(Expr::Literal(LitVal::Float(val), tok.span))
            }
            TokenKind::String(s) => {
                let val = s.clone();
                let tok = self.advance();
                if val.contains("${") {
                    self.parse_interpolated_string(&val, tok.span)
                } else {
                    Ok(Expr::Literal(LitVal::String(val), tok.span))
                }
            }
            TokenKind::True => {
                let tok = self.advance();
                Ok(Expr::Literal(LitVal::Bool(true), tok.span))
            }
            TokenKind::False => {
                let tok = self.advance();
                Ok(Expr::Literal(LitVal::Bool(false), tok.span))
            }
            TokenKind::Unit => {
                let tok = self.advance();
                Ok(Expr::Literal(LitVal::Unit, tok.span))
            }
            TokenKind::Nil => {
                let tok = self.advance();
                Ok(Expr::Literal(LitVal::Nil, tok.span))
            }
            TokenKind::Atom(s) => {
                let val = s.clone();
                let tok = self.advance();
                Ok(Expr::Atom(val, tok.span))
            }
            TokenKind::Ident(s) => {
                let val = s.clone();
                let tok = self.advance();
                Ok(Expr::Ident(val, tok.span))
            }
            TokenKind::LParen => {
                let lparen = self.advance();
                self.lexer.push_mode(LexMode::ExprMl);
                let first = self.parse_expr()?;
                if self.match_token(&TokenKind::Comma) {
                    // Tuple literal
                    let mut items = vec![first];
                    while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                        items.push(self.parse_expr()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                    let rparen = self.expect(&TokenKind::RParen)?;
                    self.lexer.pop_mode();
                    Ok(Expr::TupleLit(items, lparen.span.merge(rparen.span)))
                } else {
                    let _rparen = self.expect(&TokenKind::RParen)?;
                    self.lexer.pop_mode();
                    Ok(first)
                }
            }
            TokenKind::LBracket => {
                let lbracket = self.advance();
                self.lexer.push_mode(LexMode::ExprMl);
                let mut items = Vec::new();
                self.skip_terminators();
                while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
                    items.push(self.parse_expr()?);
                    self.skip_terminators();
                    if !self.match_token(&TokenKind::Comma) {
                        self.skip_terminators();
                        break;
                    }
                    self.skip_terminators();
                }
                self.skip_terminators();
                let rbracket = self.expect(&TokenKind::RBracket)?;
                self.lexer.pop_mode();
                Ok(Expr::ArrayLit(items, lbracket.span.merge(rbracket.span)))
            }
            TokenKind::LBrace => {
                let block = self.parse_block()?;
                Ok(Expr::Block(block))
            }
            TokenKind::If => {
                let kw = self.advance();
                let cond = self.parse_expr()?;
                if self.check(&TokenKind::Colon) {
                    self.advance();
                }
                self.skip_terminators();
                let then_branch = self.parse_block()?;
                self.skip_terminators();

                let else_branch = if self.match_token(&TokenKind::Else) {
                    if self.check(&TokenKind::Colon) {
                        self.advance();
                    }
                    self.skip_terminators();
                    if self.check(&TokenKind::If) {
                        let nested_if = self.parse_expr()?;
                        let span = nested_if.span();
                        Some(Block {
                            stmts: vec![],
                            tail_expr: Some(Box::new(nested_if)),
                            span,
                        })
                    } else {
                        Some(self.parse_block()?)
                    }
                } else {
                    None
                };

                let span = kw.span.merge(
                    else_branch
                        .as_ref()
                        .map(|b| b.span)
                        .unwrap_or(then_branch.span),
                );

                Ok(Expr::If {
                    cond: Box::new(cond),
                    then_branch,
                    else_branch,
                    span,
                })
            }
            TokenKind::Match => {
                let kw = self.advance();
                let target = self.parse_expr()?;
                if self.check(&TokenKind::Colon) {
                    self.advance();
                }
                self.skip_terminators();
                self.expect(&TokenKind::LBrace)?;
                self.lexer.push_mode(LexMode::ExprMl);

                let mut arms = Vec::new();
                while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
                    self.skip_terminators();
                    if self.check(&TokenKind::RBrace) {
                        break;
                    }
                    let pat = self.parse_pattern()?;
                    self.expect(&TokenKind::FatArrow)?;
                    let body = self.parse_expr()?;
                    let span = pat.span().merge(body.span());
                    arms.push(MatchArm {
                        pattern: pat,
                        body,
                        span,
                    });
                    if self.check(&TokenKind::Comma) || self.check(&TokenKind::Semicolon) {
                        self.advance();
                    }
                    self.skip_terminators();
                }

                let rbrace = self.expect(&TokenKind::RBrace)?;
                self.lexer.pop_mode();
                Ok(Expr::Match {
                    target: Box::new(target),
                    arms,
                    span: kw.span.merge(rbrace.span),
                })
            }
            TokenKind::Spawn => {
                let kw = self.advance();
                let callee = self.parse_primary()?;
                self.expect(&TokenKind::LParen)?;
                self.lexer.push_mode(LexMode::ExprMl);
                let mut args = Vec::new();
                while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                    args.push(self.parse_expr()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                let rparen = self.expect(&TokenKind::RParen)?;
                self.lexer.pop_mode();
                Ok(Expr::Spawn {
                    callee: Box::new(callee),
                    args,
                    span: kw.span.merge(rparen.span),
                })
            }
            TokenKind::Send => {
                let kw = self.advance();
                self.expect(&TokenKind::LParen)?;
                self.lexer.push_mode(LexMode::ExprMl);
                let receiver = self.parse_expr()?;
                self.expect(&TokenKind::Comma)?;
                let message = self.parse_expr()?;
                let rparen = self.expect(&TokenKind::RParen)?;
                self.lexer.pop_mode();
                Ok(Expr::Send {
                    receiver: Box::new(receiver),
                    message: Box::new(message),
                    span: kw.span.merge(rparen.span),
                })
            }
            TokenKind::Receive => {
                let kw = self.advance();
                self.expect(&TokenKind::LParen)?;
                let rparen = self.expect(&TokenKind::RParen)?;
                Ok(Expr::Receive(kw.span.merge(rparen.span)))
            }
            TokenKind::Fn => {
                let kw = self.advance();
                self.expect(&TokenKind::LParen)?;
                self.lexer.push_mode(LexMode::ExprMl);
                let params = self.parse_param_list()?;
                self.expect(&TokenKind::RParen)?;
                self.lexer.pop_mode();
                let ret_type = if self.match_token(&TokenKind::Arrow) {
                    Some(self.parse_type_ann()?)
                } else {
                    None
                };
                if self.check(&TokenKind::Colon) {
                    self.advance();
                }
                self.skip_terminators();
                let body = self.parse_block()?;
                let span = kw.span.merge(body.span);
                Ok(Expr::Lambda {
                    params,
                    ret_type,
                    body,
                    span,
                })
            }
            other => Err(ParseError {
                message: format!("Unexpected token in expression: '{}'", other),
                span: self.current.span,
            }),
        }
    }

    fn parse_interpolated_string(&self, s: &str, span: Span) -> Result<Expr, ParseError> {
        let mut parts = Vec::new();
        let mut last_idx = 0;
        let mut i = 0;
        let bytes = s.as_bytes();

        while i < bytes.len() {
            if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                if i > last_idx {
                    parts.push(StringPart::Literal(s[last_idx..i].to_string()));
                }
                let expr_start = i + 2;
                let mut brace_depth = 1;
                let mut expr_end = expr_start;
                let mut j = expr_start;

                while j < bytes.len() {
                    if bytes[j] == b'{' {
                        brace_depth += 1;
                    } else if bytes[j] == b'}' {
                        brace_depth -= 1;
                        if brace_depth == 0 {
                            expr_end = j;
                            break;
                        }
                    }
                    j += 1;
                }

                if brace_depth != 0 {
                    return Err(ParseError::new("Unclosed '${' in interpolated string", span));
                }

                let expr_src = &s[expr_start..expr_end];
                let expr = crate::syntax::parse_expression(expr_src)?;
                parts.push(StringPart::Expr(expr));
                last_idx = expr_end + 1;
                i = last_idx;
            } else {
                i += 1;
            }
        }

        if last_idx < s.len() {
            parts.push(StringPart::Literal(s[last_idx..].to_string()));
        }

        Ok(Expr::InterpolatedString(parts, span))
    }

    // ── Patterns ────────────────────────────────────────────────────────────

    pub fn parse_pattern(&mut self) -> Result<PatternNode, ParseError> {
        match &self.current.kind {
            TokenKind::Ident(s) if s == "_" => {
                let tok = self.advance();
                Ok(PatternNode::Wildcard(tok.span))
            }
            TokenKind::Int(i) => {
                let val = *i;
                let tok = self.advance();
                // Check if followed by .. or ..=
                if self.check(&TokenKind::DotDot) {
                    self.advance();
                    let inclusive = self.match_token(&TokenKind::Eq);
                    let end_expr = if self.check_pattern_end() {
                        None
                    } else {
                        Some(Box::new(self.parse_expr()?))
                    };
                    let span = tok.span.merge(end_expr.as_ref().map(|e| e.span()).unwrap_or(tok.span));
                    Ok(PatternNode::Range {
                        start: Some(Box::new(Expr::Literal(LitVal::Int(val), tok.span))),
                        end: end_expr,
                        inclusive,
                        span,
                    })
                } else {
                    Ok(PatternNode::Literal(LitVal::Int(val), tok.span))
                }
            }
            TokenKind::Float(bits) => {
                let val = f64::from_bits(*bits);
                let tok = self.advance();
                Ok(PatternNode::Literal(LitVal::Float(val), tok.span))
            }
            TokenKind::String(s) => {
                let val = s.clone();
                let tok = self.advance();
                Ok(PatternNode::Literal(LitVal::String(val), tok.span))
            }
            TokenKind::True => {
                let tok = self.advance();
                Ok(PatternNode::Literal(LitVal::Bool(true), tok.span))
            }
            TokenKind::False => {
                let tok = self.advance();
                Ok(PatternNode::Literal(LitVal::Bool(false), tok.span))
            }
            TokenKind::Unit => {
                let tok = self.advance();
                Ok(PatternNode::Literal(LitVal::Unit, tok.span))
            }
            TokenKind::Nil => {
                let tok = self.advance();
                Ok(PatternNode::Literal(LitVal::Nil, tok.span))
            }
            TokenKind::LParen => {
                let lparen = self.advance();
                let mut pats = Vec::new();
                while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                    pats.push(self.parse_pattern()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                let rparen = self.expect(&TokenKind::RParen)?;
                Ok(PatternNode::Tuple(pats, lparen.span.merge(rparen.span)))
            }
            TokenKind::LBracket => {
                // Array pattern with list decomposition support!
                let lbracket = self.advance();
                let mut segments = Vec::new();

                while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
                    if self.check(&TokenKind::DotDot) {
                        let dotdot = self.advance();
                        if self.check(&TokenKind::Ident("_".to_string())) {
                            let tok = self.advance();
                            segments.push(PatternSegment::WildcardList(dotdot.span.merge(tok.span)));
                        } else if let TokenKind::Ident(bind_name) = &self.current.kind {
                            let name = bind_name.clone();
                            let tok = self.advance();
                            segments.push(PatternSegment::BindList(name, dotdot.span.merge(tok.span)));
                        } else {
                            segments.push(PatternSegment::WildcardList(dotdot.span));
                        }
                    } else {
                        let pat = self.parse_pattern()?;
                        segments.push(PatternSegment::Element(pat));
                    }

                    if self.check(&TokenKind::Pipe) || self.check(&TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }

                let rbracket = self.expect(&TokenKind::RBracket)?;
                Ok(PatternNode::Array {
                    segments,
                    span: lbracket.span.merge(rbracket.span),
                })
            }
            TokenKind::Ident(s) => {
                let name = s.clone();
                let tok = self.advance();

                // Check for Enum variant match Variant(p1, p2)
                if self.check(&TokenKind::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                        args.push(self.parse_pattern()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                    let rparen = self.expect(&TokenKind::RParen)?;
                    Ok(PatternNode::EnumVariant {
                        name,
                        args,
                        span: tok.span.merge(rparen.span),
                    })
                } else {
                    Ok(PatternNode::Bind(name, tok.span))
                }
            }
            TokenKind::Atom(s) => {
                let name = s.clone();
                let tok = self.advance();
                if self.check(&TokenKind::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                        args.push(self.parse_pattern()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                    let rparen = self.expect(&TokenKind::RParen)?;
                    Ok(PatternNode::EnumVariant {
                        name,
                        args,
                        span: tok.span.merge(rparen.span),
                    })
                } else {
                    Ok(PatternNode::EnumVariant {
                        name,
                        args: vec![],
                        span: tok.span,
                    })
                }
            }
            other => Err(ParseError {
                message: format!("Unexpected token in pattern: '{}'", other),
                span: self.current.span,
            }),
        }
    }

    fn check_pattern_end(&self) -> bool {
        self.check(&TokenKind::FatArrow)
            || self.check(&TokenKind::Comma)
            || self.check(&TokenKind::Pipe)
            || self.check(&TokenKind::RBracket)
            || self.check(&TokenKind::RParen)
            || self.check(&TokenKind::Semicolon)
    }

    // ── Types ───────────────────────────────────────────────────────────────

    pub fn parse_type_ann(&mut self) -> Result<TypeAnn, ParseError> {
        match &self.current.kind {
            TokenKind::Fn => {
                let kw = self.advance();
                self.expect(&TokenKind::LParen)?;
                let mut params = Vec::new();
                while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                    params.push(self.parse_type_ann()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RParen)?;
                self.expect(&TokenKind::Arrow)?;
                let ret = self.parse_type_ann()?;
                let span = kw.span.merge(ret.span());
                Ok(TypeAnn::Fn {
                    params,
                    ret: Box::new(ret),
                    span,
                })
            }
            TokenKind::LParen => {
                let lparen = self.advance();
                let mut types = Vec::new();
                while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                    types.push(self.parse_type_ann()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                let rparen = self.expect(&TokenKind::RParen)?;
                Ok(TypeAnn::Tuple(types, lparen.span.merge(rparen.span)))
            }
            TokenKind::LBracket => {
                let lbracket = self.advance();
                let inner = self.parse_type_ann()?;
                let rbracket = self.expect(&TokenKind::RBracket)?;
                Ok(TypeAnn::Named {
                    name: "Array".to_string(),
                    args: vec![inner],
                    span: lbracket.span.merge(rbracket.span),
                })
            }
            TokenKind::Ident(name) => {
                let type_name = name.clone();
                let tok = self.advance();
                let mut args = Vec::new();
                let mut span = tok.span;

                if self.match_token(&TokenKind::LBracket) {
                    while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
                        args.push(self.parse_type_ann()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                    let rbracket = self.expect(&TokenKind::RBracket)?;
                    span = span.merge(rbracket.span);
                }

                Ok(TypeAnn::Named {
                    name: type_name,
                    args,
                    span,
                })
            }
            other => Err(ParseError {
                message: format!("Expected type annotation, found '{}'", other),
                span: self.current.span,
            }),
        }
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

    fn parse_ident_name(&mut self) -> Result<String, ParseError> {
        if let TokenKind::Ident(s) = &self.current.kind {
            let name = s.clone();
            self.advance();
            Ok(name)
        } else {
            Err(ParseError {
                message: format!("Expected identifier, found '{}'", self.current.kind),
                span: self.current.span,
            })
        }
    }

    fn expect_ident(&mut self) -> Result<Token, ParseError> {
        if let TokenKind::Ident(_) = &self.current.kind {
            Ok(self.advance())
        } else {
            Err(ParseError {
                message: format!("Expected identifier, found '{}'", self.current.kind),
                span: self.current.span,
            })
        }
    }

    fn parse_opt_type_params(&mut self) -> Result<Vec<String>, ParseError> {
        if self.match_token(&TokenKind::LBracket) {
            let mut params = Vec::new();
            while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
                params.push(self.parse_ident_name()?);
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(&TokenKind::RBracket)?;
            Ok(params)
        } else {
            Ok(vec![])
        }
    }

    fn parse_param_list(&mut self) -> Result<Vec<Param>, ParseError> {
        let mut params = Vec::new();
        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
            let name = self.parse_ident_name()?;
            let type_ann = if self.match_token(&TokenKind::Colon) {
                Some(self.parse_type_ann()?)
            } else {
                None
            };
            params.push(Param {
                name,
                type_ann,
                span: self.current.span,
            });
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        Ok(params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn test_parse_simple_fn() {
        let src = r#"
        fn add(a: Int, b: Int) -> Int: {
            return a + b;
        }
        "#;
        let module = parse(src).expect("parsed");
        assert_eq!(module.items.len(), 1);
        if let Item::Decl(Decl::Fn(f)) = &module.items[0] {
            assert_eq!(f.name, "add");
            assert_eq!(f.params.len(), 2);
            assert_eq!(f.body.stmts.len(), 1);
        } else {
            panic!("expected fn");
        }
    }

    #[test]
    fn test_parse_match_with_array_decomposition() {
        let src = r#"
        fn classify(arr: Array[Int]) -> Int: {
            match arr: {
                [..a | b] => b,
                [head | ..tail] => head,
                [a, b | ..c] => a + b,
                [5..10] => 1,
                _ => 0,
            }
        }
        "#;
        let module = parse(src).expect("parsed");
        assert_eq!(module.items.len(), 1);
    }

    #[test]
    fn test_parse_struct_and_enum() {
        let src = r#"
        struct Point: {
            x: Float,
            y: Float,
        }

        enum Shape: {
            Circle(Float),
            Rect(Float, Float),
        }
        "#;
        let module = parse(src).expect("parsed");
        assert_eq!(module.items.len(), 2);
    }
}

