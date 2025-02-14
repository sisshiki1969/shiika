mod hir_maker;
mod hir_maker_context;
mod index;
use std::collections::HashMap;
use crate::ast;
use crate::ty;
use crate::ty::*;
use crate::names::*;
use crate::stdlib::Stdlib;

pub struct Hir {
    pub sk_classes: HashMap<ClassFullname, SkClass>,
    pub sk_methods: HashMap<ClassFullname, Vec<SkMethod>>,
    pub constants: HashMap<ConstFullname, TermTy>,
    pub main_exprs: HirExpressions,
}
impl Hir {
    pub fn from_ast(ast: ast::Program, stdlib: Stdlib) -> Result<Hir, crate::error::Error> {
        let index = index::Index::new(stdlib.sk_classes, &ast.toplevel_defs)?;
        let mut hir = hir_maker::HirMaker::convert_program(index, ast)?;

        // While stdlib classes are included in `index`,
        // stdlib methods are not. Here we need to add them manually
        hir.add_methods(stdlib.sk_methods);

        Ok(hir)
    }

    pub fn add_classes(&mut self, sk_classes: HashMap<ClassFullname, SkClass>) {
        self.sk_classes.extend(sk_classes)
    }

    pub fn add_methods(&mut self, sk_methods: HashMap<ClassFullname, Vec<SkMethod>>) {
        self.sk_methods.extend(sk_methods)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct SkClass {
    pub fullname: ClassFullname,
    pub superclass_fullname: Option<ClassFullname>,
    pub instance_ty: TermTy,
    pub method_sigs: HashMap<MethodFirstname, MethodSignature>,
}
impl SkClass {
    pub fn class_ty(&self) -> TermTy {
        self.instance_ty.meta_ty()
    }
}

#[derive(Debug, PartialEq)]
pub struct SkMethod {
    pub signature: MethodSignature,
    pub body: SkMethodBody,
}

pub enum SkMethodBody {
    ShiikaMethodBody {
        exprs: HirExpressions
    },
    RustMethodBody {
        gen: GenMethodBody
    },
    RustClosureMethodBody {
        boxed_gen: Box<ClosureMethodBody>
    }
}
// Manually deriving because GenMethodBody is a function (auto-deriving seems unsupported)
impl std::fmt::Debug for SkMethodBody {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "#<SkMethodBody>")
    }
}
impl std::cmp::PartialEq for SkMethodBody {
    fn eq(&self, other: &SkMethodBody) -> bool {
        match self {
            SkMethodBody::ShiikaMethodBody { exprs } => {
                match other {
                    SkMethodBody::ShiikaMethodBody { exprs: exprs2 } => return exprs == exprs2,
                    SkMethodBody::RustMethodBody { .. } => (),
                    SkMethodBody::RustClosureMethodBody { .. } => (),
                }
            },
            SkMethodBody::RustMethodBody { .. } => (),
            SkMethodBody::RustClosureMethodBody { .. } => (),
        }
        panic!("cannot compare RustMethodBody");
    }
}

pub type GenMethodBody = fn(code_gen: &crate::code_gen::CodeGen, function: &inkwell::values::FunctionValue) -> Result<(), crate::error::Error>;
pub type ClosureMethodBody = dyn Fn(&crate::code_gen::CodeGen, &inkwell::values::FunctionValue) -> Result<(), crate::error::Error>;

#[derive(Debug, PartialEq)]
pub struct HirExpressions {
    pub ty: TermTy,
    pub exprs: Vec<HirExpression>,
}

#[derive(Debug, PartialEq)]
pub struct HirExpression {
    pub ty: TermTy,
    pub node: HirExpressionBase,
}

#[derive(Debug, PartialEq)]
pub enum HirExpressionBase {
    HirIfExpression {
        cond_expr: Box<HirExpression>,
        then_expr: Box<HirExpression>,
        else_expr: Box<HirExpression>,
    },
    HirWhileExpression {
        cond_expr: Box<HirExpression>,
        body_exprs: Box<HirExpressions>,
    },
    HirLVarAssign {
        name: String,
        rhs: Box<HirExpression>,
    },
    HirConstAssign {
        fullname: ConstFullname,
        rhs: Box<HirExpression>,
    },
    HirMethodCall {
        receiver_expr: Box<HirExpression>,
        method_fullname: MethodFullname,
        arg_exprs: Vec<HirExpression>,
    },
    HirArgRef {
        idx: usize,
    },
    HirLVarRef {
        name: String,
    },
    HirConstRef {
        fullname: ConstFullname,
    },
    HirSelfExpression,
    HirFloatLiteral {
        value: f64,
    },
    HirDecimalLiteral {
        value: i32,
    },
    HirBooleanLiteral {
        value: bool,
    },
    /// A special expression that evaluates to a class
    /// (eg. `class A; end; A = 1` shadows A, but this special expr
    /// is never be shadowed)
    HirClassLiteral {
        fullname: ClassFullname,
    },
    HirNop  // For else-less if expr
}

impl Hir {
    pub fn if_expression(ty: TermTy,
                         cond_hir: HirExpression,
                         then_hir: HirExpression,
                         else_hir: HirExpression) -> HirExpression {
        HirExpression {
            ty: ty,
            node: HirExpressionBase::HirIfExpression {
                cond_expr: Box::new(cond_hir),
                then_expr: Box::new(then_hir),
                else_expr: Box::new(else_hir),
            }
        }
    }

    pub fn while_expression(cond_hir: HirExpression,
                            body_hirs: HirExpressions) -> HirExpression {
        HirExpression {
            ty: ty::raw("Void"),
            node: HirExpressionBase::HirWhileExpression {
                cond_expr: Box::new(cond_hir),
                body_exprs: Box::new(body_hirs),
            }
        }
    }

    pub fn assign_lvar(name: &str, rhs: HirExpression) -> HirExpression {
        HirExpression {
            ty: rhs.ty.clone(),
            node: HirExpressionBase::HirLVarAssign {
                name: name.to_string(),
                rhs: Box::new(rhs),
            }
        }
    }

    pub fn assign_const(fullname: ConstFullname, rhs: HirExpression) -> HirExpression {
        HirExpression {
            ty: rhs.ty.clone(),
            node: HirExpressionBase::HirConstAssign {
                fullname: fullname,
                rhs: Box::new(rhs),
            }
        }
    }

    pub fn method_call(result_ty: TermTy, receiver_hir: HirExpression, method_fullname: MethodFullname, arg_hirs: Vec<HirExpression>) -> HirExpression {
        HirExpression {
            ty: result_ty,
            node: HirExpressionBase::HirMethodCall {
                receiver_expr: Box::new(receiver_hir),
                method_fullname: method_fullname,
                arg_exprs: arg_hirs,
            }
        }
    }

    // REFACTOR: Remove `hir_`
    pub fn hir_arg_ref(ty: TermTy, idx: usize) -> HirExpression {
        HirExpression {
            ty: ty,
            node: HirExpressionBase::HirArgRef { idx: idx },
        }
    }

    pub fn lvar_ref(ty: TermTy, name: String) -> HirExpression {
        HirExpression {
            ty: ty,
            node: HirExpressionBase::HirLVarRef { name },
        }
    }

    pub fn const_ref(ty: TermTy, fullname: ConstFullname) -> HirExpression {
        HirExpression {
            ty: ty,
            node: HirExpressionBase::HirConstRef { fullname },
        }
    }

    pub fn self_expression(ty: TermTy) -> HirExpression {
        HirExpression {
            ty: ty,
            node: HirExpressionBase::HirSelfExpression,
        }
    }

    pub fn float_literal(value: f64) -> HirExpression {
        HirExpression {
            ty: ty::raw("Float"),
            node: HirExpressionBase::HirFloatLiteral { value }
        }
    }
    
    pub fn decimal_literal(value: i32) -> HirExpression {
        HirExpression {
            ty: ty::raw("Int"),
            node: HirExpressionBase::HirDecimalLiteral { value }
        }
    }
    
    pub fn boolean_literal(value: bool) -> HirExpression {
        HirExpression {
            ty: ty::raw("Bool"),
            node: HirExpressionBase::HirBooleanLiteral { value }
        }
    }

    pub fn class_literal(fullname: ClassFullname) -> HirExpression {
        HirExpression {
            ty: ty::meta(&fullname.0),
            node: HirExpressionBase::HirClassLiteral { fullname }
        }
    }
    
    pub fn nop() -> HirExpression {
        HirExpression {
            ty: ty::raw(" NOP "), // must not be used
            node: HirExpressionBase::HirNop,
        }
    }
}

/// Create `hir::MethodSignature` from `ast::MethodSignature`
pub fn create_signature(class_fullname: String, sig: &ast::AstMethodSignature) -> MethodSignature {
    let fullname = MethodFullname {
        full_name: (class_fullname + "#" + &sig.name.0),
        first_name: sig.name.clone(),
    };
    let ret_ty = convert_typ(&sig.ret_typ);
    let params = sig.params.iter().map(|param|
        MethodParam { name: param.name.to_string(), ty: convert_typ(&param.typ) }
    ).collect();

    MethodSignature { fullname, ret_ty, params }
}

fn convert_typ(typ: &ast::Typ) -> TermTy {
    ty::raw(&typ.name)
}

/// Create a signature of `.new`
fn signature_of_new(metaclass_fullname: &ClassFullname, instance_ty: &TermTy) -> MethodSignature {
    MethodSignature {
        fullname: MethodFullname {
            full_name: metaclass_fullname.0.clone() + "#new",
            first_name: MethodFirstname("new".to_string()),
        },
        ret_ty: instance_ty.clone(),
        params: vec![],
    }
}
