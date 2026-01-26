use crate::ast::{BinaryOp, Expr, ExprX, Ral, RalEntry};
use codegen::Scope;
use std::collections::HashMap;

// Evaluate constant expression. Returns None if it depends on unknown variables.
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

fn convert_expr_to_string(expr: &Expr, defines: &HashMap<String, u64>) -> String {
    if let Some(val) = eval_const(expr, defines) {
        return format!("{}", val);
    }
    
    match &**expr {
        ExprX::Num(n) => n.to_string(),
        ExprX::Var(v) => {
            if let Some(val) = defines.get(v.as_str()) {
                val.to_string()
            } else {
                v.to_string()
            }
        },
        ExprX::Binary(op, l, r) => {
            let l_str = convert_expr_to_string(&l.x, defines);
            let r_str = convert_expr_to_string(&r.x, defines);
            let op_str = match op {
                BinaryOp::Add => "+",
                BinaryOp::Subtract => "-",
            };
            format!("({}) {} ({})", l_str, op_str, r_str)
        }
    }
}

pub fn convert_to_rust(ral: Ral, defines: &HashMap<String, u64>) -> String {
    let mut scope = Scope::new();

    for (reg_name, entry) in &ral.registers {
        if let RalEntry::RawRegister(reg_spanned) = entry {
            let reg = &reg_spanned.x;
            
            // We'll generate a module for the register if we wanted to group them, 
            // but following the C pattern, we'll generate flat functions with prefixes.
            // Or maybe better: generated struct impls?
            // The prompt asked for "setter and getter for the fields".
            // Let's stick to simple functions for now to match C output style requested earlier,
            // but mostly just valid Rust.
            
            let mut offset_const: u64 = 0;
            let mut offset_dynamic: Vec<String> = Vec::new();
            
            // Iterate fields in reverse (LSB first)
            for field in reg.fields.iter().rev() {
                // Calculate current offset string
                let offset_str = if offset_dynamic.is_empty() {
                    offset_const.to_string()
                } else {
                    let mut parts = offset_dynamic.clone();
                    if offset_const > 0 {
                        parts.push(offset_const.to_string());
                    }
                    parts.join(" + ")
                };

                let mask_val_opt = eval_const(&field.size.x, defines).map(|size| {
                    if size < 64 {
                        (1u64 << size) - 1
                    } else {
                        u64::MAX // Should probably handle full width properly
                    }
                });
                
                let mask_str = if let Some(mask) = mask_val_opt {
                    format!("0x{:x}", mask)
                } else {
                    let size_str = convert_expr_to_string(&field.size.x, defines);
                    format!("(1 << ({})) - 1", size_str)
                };

                if let Some(field_name_spanned) = &field.name {
                    let field_name = &field_name_spanned.x;
                    
                    // Getter
                    let get_name = format!("{}_{}_get", reg_name, field_name);
                    let get_fn = scope.new_fn(&get_name)
                        .vis("pub")
                        .arg("val", "u64")
                        .ret("u64");
                    
                    get_fn.line(format!("(val >> {}) & {}", offset_str, mask_str));

                    // Setter
                    let set_name = format!("{}_{}_set", reg_name, field_name);
                    let set_fn = scope.new_fn(&set_name)
                        .vis("pub")
                        .arg("val", "u64")
                        .arg("field_val", "u64")
                        .ret("u64");
                    
                    set_fn.line(format!("(val & !({} << {})) | ((field_val & {}) << {})", 
                        mask_str, offset_str, mask_str, offset_str));
                }

                // Update offset state
                if let Some(n) = eval_const(&field.size.x, defines) {
                    offset_const += n;
                } else {
                    offset_dynamic.push(convert_expr_to_string(&field.size.x, defines));
                }
            }
        }
    }

    scope.to_string()
}