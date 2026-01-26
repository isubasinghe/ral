use crate::ast::{self, BinaryOp, Expr, ExprX, Ral, RalEntry};
use lang_c::ast::*;
use lang_c::span::{Node, Span};
use std::collections::HashMap;

fn node<T>(x: T) -> Node<T> {
    Node::new(x, Span::none())
}

fn make_id(name: &str) -> Node<Identifier> {
    node(Identifier {
        name: name.into(),
    })
}

fn make_constant(n: u64) -> Node<Expression> {
    node(Expression::Constant(Box::new(node(Constant::Integer(Integer {
        base: IntegerBase::Decimal,
        suffix: IntegerSuffix {
            size: IntegerSize::LongLong,
            unsigned: true,
            imaginary: false,
        },
        number: n.to_string().into_boxed_str(),
    }))))) 
}

fn convert_expr(expr: &Expr, defines: &HashMap<String, u64>) -> Node<Expression> {
    match &**expr {
        ExprX::Num(n) => make_constant(*n as u64),
        ExprX::Var(v) => {
            if let Some(val) = defines.get(v.as_str()) {
                make_constant(*val)
            } else {
                node(Expression::Identifier(Box::new(make_id(v))))
            }
        },
        ExprX::Binary(op, l, r) => {
            let op = match op {
                BinaryOp::Add => BinaryOperator::Plus,
                BinaryOp::Subtract => BinaryOperator::Minus,
            };
            node(Expression::BinaryOperator(Box::new(node(BinaryOperatorExpression {
                operator: node(op),
                lhs: Box::new(convert_expr(&l.x, defines)),
                rhs: Box::new(convert_expr(&r.x, defines)),
            }))))
        }
    }
}

// Generate (1ULL << size) - 1
// Optimized: if size is constant, compute value
fn make_mask(size_expr: &Expr, defines: &HashMap<String, u64>) -> Node<Expression> {
    // Helper to evaluate constant expression if possible
    fn eval_const(expr: &Expr, defines: &HashMap<String, u64>) -> Option<u64> {
        match &**expr {
            ExprX::Num(n) => Some(*n as u64),
            ExprX::Var(v) => defines.get(v.as_str()).copied(),
            ExprX::Binary(op, l, r) => {
                let l_val = eval_const(&l.x, defines)?;
                let r_val = eval_const(&r.x, defines)?;
                match op {
                    BinaryOp::Add => Some(l_val.wrapping_add(r_val)),
                    BinaryOp::Subtract => Some(l_val.wrapping_sub(r_val)),
                }
            }
        }
    }

    if let Some(n) = eval_const(size_expr, defines) {
        if n < 64 {
            let val = (1u64 << n) - 1;
            // Use hex for cleaner output if > 9
            let num_str = if val > 9 {
                format!("0x{:x}", val)
            } else {
                val.to_string()
            };
            
            return node(Expression::Constant(Box::new(node(Constant::Integer(Integer {
                base: if val > 9 { IntegerBase::Hexadecimal } else { IntegerBase::Decimal },
                suffix: IntegerSuffix {
                    size: IntegerSize::LongLong,
                    unsigned: true,
                    imaginary: false,
                },
                number: num_str.into_boxed_str(),
            })))));
        }
    }

    let size_expr_node = convert_expr(size_expr, defines);
    let one = make_constant(1);
    
    // (1 << size)
    let shift = node(Expression::BinaryOperator(Box::new(node(BinaryOperatorExpression {
        operator: node(BinaryOperator::ShiftLeft),
        lhs: Box::new(one.clone()),
        rhs: Box::new(size_expr_node),
    }))));
    
    // (1 << size) - 1
    node(Expression::BinaryOperator(Box::new(node(BinaryOperatorExpression {
        operator: node(BinaryOperator::Minus),
        lhs: Box::new(shift),
        rhs: Box::new(one),
    }))))
}

// Declarator helper: static inline uint64_t name(params)
fn make_function_def(name: String, params: Vec<(String, String)>, body: Vec<Node<BlockItem>>) -> Node<ExternalDeclaration> {
    // Specifiers: static inline uint64_t
    let specifiers = vec![
        node(DeclarationSpecifier::StorageClass(node(StorageClassSpecifier::Static))),
        node(DeclarationSpecifier::Function(node(FunctionSpecifier::Inline))),
        node(DeclarationSpecifier::TypeSpecifier(node(TypeSpecifier::TypedefName(make_id("uint64_t"))))),
    ];

    // Param declarations
    let mut param_decls = Vec::new();
    for (p_type, p_name) in params {
        let decl = node(ParameterDeclaration {
            specifiers: vec![node(DeclarationSpecifier::TypeSpecifier(node(TypeSpecifier::TypedefName(make_id(&p_type)))))],
            declarator: Some(node(Declarator {
                kind: node(DeclaratorKind::Identifier(make_id(&p_name))),
                derived: vec![],
                extensions: vec![],
            })),
            extensions: vec![],
        });
        param_decls.push(decl);
    }

    // Declarator: name(params)
    let func_declarator = node(FunctionDeclarator {
        parameters: param_decls,
        ellipsis: Ellipsis::None,
    });

    let declarator = node(Declarator {
        kind: node(DeclaratorKind::Identifier(make_id(&name))),
        derived: vec![node(DerivedDeclarator::Function(func_declarator))],
        extensions: vec![],
    });

    // Body
    let compound_stmt = node(Statement::Compound(body));

    node(ExternalDeclaration::FunctionDefinition(node(FunctionDefinition {
        specifiers,
        declarator,
        declarations: vec![],
        statement: compound_stmt,
    })))
}

// Custom Code Printer
struct CodePrinter<'a> {
    w: &'a mut String,
    indent: usize,
}

impl<'a> CodePrinter<'a> {
    fn new(w: &'a mut String) -> Self {
        Self { w, indent: 0 }
    }

    fn print_tu(&mut self, unit: &TranslationUnit) {
        for decl in &unit.0 {
            self.print_external_decl(&decl.node);
            self.w.push('\n');
        }
    }

    fn print_external_decl(&mut self, decl: &ExternalDeclaration) {
        match decl {
            ExternalDeclaration::FunctionDefinition(node) => self.print_func_def(&node.node),
            _ => {},
        }
    }

    fn print_func_def(&mut self, def: &FunctionDefinition) {
        for spec in &def.specifiers {
            self.print_decl_specifier(&spec.node);
            self.w.push(' ');
        }
        
        self.print_declarator(&def.declarator.node);
        self.w.push(' ');
        
        self.print_statement(&def.statement.node);
        self.w.push('\n');
    }

    fn print_decl_specifier(&mut self, spec: &DeclarationSpecifier) {
        match spec {
            DeclarationSpecifier::StorageClass(s) => match s.node {
                StorageClassSpecifier::Static => self.w.push_str("static"),
                _ => {},
            },
            DeclarationSpecifier::Function(f) => match f.node {
                FunctionSpecifier::Inline => self.w.push_str("inline"),
                _ => {},
            },
            DeclarationSpecifier::TypeSpecifier(t) => self.print_type_specifier(&t.node),
            _ => {},
        }
    }

    fn print_type_specifier(&mut self, t: &TypeSpecifier) {
        match t {
            TypeSpecifier::TypedefName(id) => self.w.push_str(&id.node.name),
            _ => {},
        }
    }

    fn print_declarator(&mut self, decl: &Declarator) {
        match &decl.kind.node {
            DeclaratorKind::Identifier(id) => self.w.push_str(&id.node.name),
            _ => {},
        }
        
        for derived in &decl.derived {
            match &derived.node {
                DerivedDeclarator::Function(func) => {
                    self.w.push('(');
                    for (i, param) in func.node.parameters.iter().enumerate() {
                        if i > 0 { self.w.push_str(", "); }
                        self.print_param_decl(&param.node);
                    }
                    self.w.push(')');
                },
                _ => {}
            }
        }
    }
    
    fn print_param_decl(&mut self, param: &ParameterDeclaration) {
        for spec in &param.specifiers {
            self.print_decl_specifier(&spec.node);
            self.w.push(' ');
        }
        if let Some(decl) = &param.declarator {
            self.print_declarator(&decl.node);
        }
    }

    fn print_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Compound(items) => {
                self.w.push_str("{\n");
                self.indent += 1;
                for item in items {
                    match &item.node {
                        BlockItem::Statement(s) => {
                             self.indent_line();
                             self.print_statement(&s.node);
                             if matches!(s.node, Statement::Return(_)) {
                                 self.w.push(';');
                             }
                             self.w.push('\n');
                        },
                        _ => {}
                    }
                }
                self.indent -= 1;
                self.indent_line();
                self.w.push('}');
            },
            Statement::Return(expr_opt) => {
                self.w.push_str("return");
                if let Some(expr) = expr_opt {
                    self.w.push(' ');
                    self.print_expr(&expr.node);
                }
            },
            _ => {}
        }
    }

    fn indent_line(&mut self) {
        for _ in 0..self.indent {
            self.w.push_str("    ");
        }
    }

    fn print_expr(&mut self, expr: &Expression) {
        match expr {
            Expression::Constant(c) => {
                 match &c.node {
                     Constant::Integer(i) => {
                         self.w.push_str(&i.number);
                         if let IntegerSuffix { size: IntegerSize::LongLong, unsigned: true, .. } = i.suffix {
                             self.w.push_str("ULL");
                         }
                     },
                     _ => {}
                 }
            },
            Expression::Identifier(id) => self.w.push_str(&id.node.name),
            Expression::BinaryOperator(bin) => {
                self.w.push('(');
                self.print_expr(&bin.node.lhs.node);
                self.w.push(' ');
                self.print_binop(&bin.node.operator.node);
                self.w.push(' ');
                self.print_expr(&bin.node.rhs.node);
                self.w.push(')');
            },
            Expression::UnaryOperator(u) => {
                if let UnaryOperator::Complement = u.node.operator.node {
                    self.w.push('~');
                }
                self.w.push('(');
                self.print_expr(&u.node.operand.node);
                self.w.push(')');
            }
            _ => {}
        }
    }

    fn print_binop(&mut self, op: &BinaryOperator) {
         match op {
             BinaryOperator::Plus => self.w.push('+'),
             BinaryOperator::Minus => self.w.push('-'),
             BinaryOperator::ShiftLeft => self.w.push_str("<<"),
             BinaryOperator::ShiftRight => self.w.push_str(">>"),
             BinaryOperator::BitwiseAnd => self.w.push('&'),
             BinaryOperator::BitwiseOr => self.w.push('|'),
             _ => {},
         }
    }
}

pub fn convert_to_c(ral: Ral, defines: &HashMap<String, u64>) -> String {
    let mut ext_decls = Vec::new();

    for (reg_name, entry) in &ral.registers {
        if let RalEntry::RawRegister(reg_spanned) = entry {
            let reg = &reg_spanned.x;
            
            // Track offset as (const_part, dynamic_parts)
            let mut offset_const: u64 = 0;
            let mut offset_dynamic: Vec<Node<Expression>> = Vec::new();
            
            // We need to iterate in reverse
            for field in reg.fields.iter().rev() {
                // Construct current offset expression
                let current_offset = if offset_dynamic.is_empty() {
                    make_constant(offset_const)
                } else {
                    let mut expr = offset_dynamic[0].clone();
                    for part in &offset_dynamic[1..] {
                        expr = node(Expression::BinaryOperator(Box::new(node(BinaryOperatorExpression {
                            operator: node(BinaryOperator::Plus),
                            lhs: Box::new(expr),
                            rhs: Box::new(part.clone()),
                        }))));
                    }
                    if offset_const > 0 {
                        expr = node(Expression::BinaryOperator(Box::new(node(BinaryOperatorExpression {
                            operator: node(BinaryOperator::Plus),
                            lhs: Box::new(expr),
                            rhs: Box::new(make_constant(offset_const)),
                        }))));
                    }
                    expr
                };

                let mask_expr = make_mask(&field.size.x, defines);
                
                // If field has a name, generate getters/setters
                if let Some(field_name_spanned) = &field.name {
                    let field_name = &field_name_spanned.x;
                    
                    // Getter: uint64_t reg_field_get(uint64_t val)
                    // return (val >> offset) & mask;
                    let get_name = format!("{}_{}_get", reg_name, field_name);
                    
                    let val_expr = node(Expression::Identifier(Box::new(make_id("val"))));
                    
                    // (val >> offset)
                    let shift_expr = node(Expression::BinaryOperator(Box::new(node(BinaryOperatorExpression {
                        operator: node(BinaryOperator::ShiftRight),
                        lhs: Box::new(val_expr.clone()),
                        rhs: Box::new(current_offset.clone()),
                    }))));
                    
                    // & mask
                    let result_expr = node(Expression::BinaryOperator(Box::new(node(BinaryOperatorExpression {
                        operator: node(BinaryOperator::BitwiseAnd),
                        lhs: Box::new(shift_expr),
                        rhs: Box::new(mask_expr.clone()),
                    }))));
                    
                    let return_stmt = node(BlockItem::Statement(node(Statement::Return(Some(Box::new(result_expr))))));
                    
                    ext_decls.push(make_function_def(
                        get_name, 
                        vec![("uint64_t".into(), "val".into())],
                        vec![return_stmt]
                    ));

                    // Setter: uint64_t reg_field_set(uint64_t val, uint64_t field_val)
                    // return (val & ~(mask << offset)) | ((field_val & mask) << offset);
                    let set_name = format!("{}_{}_set", reg_name, field_name);
                    let field_val_expr = node(Expression::Identifier(Box::new(make_id("field_val"))));

                    // mask << offset
                    let mask_shifted = node(Expression::BinaryOperator(Box::new(node(BinaryOperatorExpression {
                        operator: node(BinaryOperator::ShiftLeft),
                        lhs: Box::new(mask_expr.clone()),
                        rhs: Box::new(current_offset.clone()),
                    }))));

                    // ~(mask << offset)
                    let not_mask_shifted = node(Expression::UnaryOperator(Box::new(node(UnaryOperatorExpression {
                        operator: node(UnaryOperator::Complement),
                        operand: Box::new(mask_shifted),
                    }))));

                    // val & ~(...)
                    let val_cleared = node(Expression::BinaryOperator(Box::new(node(BinaryOperatorExpression {
                        operator: node(BinaryOperator::BitwiseAnd),
                        lhs: Box::new(val_expr.clone()),
                        rhs: Box::new(not_mask_shifted),
                    }))));

                    // (field_val & mask)
                    let val_masked = node(Expression::BinaryOperator(Box::new(node(BinaryOperatorExpression {
                        operator: node(BinaryOperator::BitwiseAnd),
                        lhs: Box::new(field_val_expr),
                        rhs: Box::new(mask_expr.clone()),
                    }))));

                    // (...) << offset
                    let val_shifted = node(Expression::BinaryOperator(Box::new(node(BinaryOperatorExpression {
                        operator: node(BinaryOperator::ShiftLeft),
                        lhs: Box::new(val_masked),
                        rhs: Box::new(current_offset.clone()),
                    }))));

                    // result | ...
                    let result_final = node(Expression::BinaryOperator(Box::new(node(BinaryOperatorExpression {
                        operator: node(BinaryOperator::BitwiseOr),
                        lhs: Box::new(val_cleared),
                        rhs: Box::new(val_shifted),
                    }))));

                    let return_stmt_set = node(BlockItem::Statement(node(Statement::Return(Some(Box::new(result_final))))));

                    ext_decls.push(make_function_def(
                        set_name,
                        vec![("uint64_t".into(), "val".into()), ("uint64_t".into(), "field_val".into())],
                        vec![return_stmt_set]
                    ));
                }
                
                // Update offset state
                // We should assume that if we can resolve it to constant, we do
                
                // Helper to resolve constant for state update
                fn eval_const_state(expr: &Expr, defines: &HashMap<String, u64>) -> Option<u64> {
                    match &**expr {
                        ExprX::Num(n) => Some(*n as u64),
                        ExprX::Var(v) => defines.get(v.as_str()).copied(),
                        ExprX::Binary(op, l, r) => {
                            let l_val = eval_const_state(&l.x, defines)?;
                            let r_val = eval_const_state(&r.x, defines)?;
                            match op {
                                BinaryOp::Add => Some(l_val.wrapping_add(r_val)),
                                BinaryOp::Subtract => Some(l_val.wrapping_sub(r_val)),
                            }
                        }
                    }
                }

                if let Some(n) = eval_const_state(&field.size.x, defines) {
                    offset_const += n;
                } else {
                    offset_dynamic.push(convert_expr(&field.size.x, defines));
                }
            }
        }
    }

    let unit = TranslationUnit(ext_decls);
    
    let mut output = String::new();
    // Prepend header
    output.push_str("#include <stdint.h>\n\n");
    
    let mut printer = CodePrinter::new(&mut output);
    printer.print_tu(&unit);
    
    output
}