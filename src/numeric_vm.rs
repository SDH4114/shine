use std::collections::HashMap;

use crate::{
    ast::{
        AssignOp, AssignTarget, BinaryOp, Block, Expr, LoopKind, Param, Program, Stmt, TypeRef,
        UnaryOp,
    },
    token::Span,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NumericType {
    Int,
    Float,
    IntList,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum NumericParameter {
    Int(usize),
    Float(usize),
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum MathOp {
    Abs,
    Floor,
    Ceil,
    Sqrt,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Log,
    Log10,
    Log2,
    Exp,
    Exp2,
    Cbrt,
    Trunc,
    Fract,
    Sinh,
    Cosh,
    Tanh,
    Asinh,
    Acosh,
    Atanh,
    Degrees,
    Radians,
}

impl MathOp {
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "abs" => Self::Abs,
            "floor" => Self::Floor,
            "ceil" => Self::Ceil,
            "sqrt" => Self::Sqrt,
            "sin" => Self::Sin,
            "cos" => Self::Cos,
            "tan" => Self::Tan,
            "asin" => Self::Asin,
            "acos" => Self::Acos,
            "atan" => Self::Atan,
            "log" => Self::Log,
            "log10" => Self::Log10,
            "log2" => Self::Log2,
            "exp" => Self::Exp,
            "exp2" => Self::Exp2,
            "cbrt" => Self::Cbrt,
            "trunc" => Self::Trunc,
            "fract" => Self::Fract,
            "sinh" => Self::Sinh,
            "cosh" => Self::Cosh,
            "tanh" => Self::Tanh,
            "asinh" => Self::Asinh,
            "acosh" => Self::Acosh,
            "atanh" => Self::Atanh,
            "degrees" => Self::Degrees,
            "radians" => Self::Radians,
            _ => return None,
        })
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Abs => "abs",
            Self::Floor => "floor",
            Self::Ceil => "ceil",
            Self::Sqrt => "sqrt",
            Self::Sin => "sin",
            Self::Cos => "cos",
            Self::Tan => "tan",
            Self::Asin => "asin",
            Self::Acos => "acos",
            Self::Atan => "atan",
            Self::Log => "log",
            Self::Log10 => "log10",
            Self::Log2 => "log2",
            Self::Exp => "exp",
            Self::Exp2 => "exp2",
            Self::Cbrt => "cbrt",
            Self::Trunc => "trunc",
            Self::Fract => "fract",
            Self::Sinh => "sinh",
            Self::Cosh => "cosh",
            Self::Tanh => "tanh",
            Self::Asinh => "asinh",
            Self::Acosh => "acosh",
            Self::Atanh => "atanh",
            Self::Degrees => "degrees",
            Self::Radians => "radians",
        }
    }

    #[inline(always)]
    pub(crate) fn apply(self, value: f64) -> f64 {
        match self {
            Self::Abs => value.abs(),
            Self::Floor => value.floor(),
            Self::Ceil => value.ceil(),
            Self::Sqrt => value.sqrt(),
            Self::Sin => value.sin(),
            Self::Cos => value.cos(),
            Self::Tan => value.tan(),
            Self::Asin => value.asin(),
            Self::Acos => value.acos(),
            Self::Atan => value.atan(),
            Self::Log => value.ln(),
            Self::Log10 => value.log10(),
            Self::Log2 => value.log2(),
            Self::Exp => value.exp(),
            Self::Exp2 => value.exp2(),
            Self::Cbrt => value.cbrt(),
            Self::Trunc => value.trunc(),
            Self::Fract => value.fract(),
            Self::Sinh => value.sinh(),
            Self::Cosh => value.cosh(),
            Self::Tanh => value.tanh(),
            Self::Asinh => value.asinh(),
            Self::Acosh => value.acosh(),
            Self::Atanh => value.atanh(),
            Self::Degrees => value.to_degrees(),
            Self::Radians => value.to_radians(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum IntExpr {
    Literal(i64),
    Local(usize),
    Global(String, Span),
    Negate(Box<Self>, Span),
    Add(Box<Self>, Box<Self>, Span),
    Subtract(Box<Self>, Box<Self>, Span),
    Multiply(Box<Self>, Box<Self>, Span),
    IntegerDivide(Box<Self>, Box<Self>, Span),
    Remainder(Box<Self>, Box<Self>, Span),
    ListIndex(usize, Box<Self>, Span),
    ListLength(usize),
}

#[derive(Clone, Debug)]
pub(crate) enum FloatExpr {
    Literal(f64),
    Local(usize),
    Global(String, Span),
    FromInt(Box<IntExpr>),
    Negate(Box<Self>),
    Add(Box<Self>, Box<Self>),
    Subtract(Box<Self>, Box<Self>),
    Multiply(Box<Self>, Box<Self>),
    Divide(Box<Self>, Box<Self>, Span),
    IntegerDivide(Box<Self>, Box<Self>, Span),
    Remainder(Box<Self>, Box<Self>, Span),
    Power(Box<Self>, Box<Self>),
    Math(MathOp, Box<Self>, Span),
}

#[derive(Clone, Debug)]
pub(crate) enum NumberExpr {
    Int(IntExpr),
    Float(FloatExpr),
}

impl NumberExpr {
    fn ty(&self) -> NumericType {
        match self {
            Self::Int(_) => NumericType::Int,
            Self::Float(_) => NumericType::Float,
        }
    }

    fn into_float(self) -> FloatExpr {
        match self {
            Self::Int(value) => FloatExpr::FromInt(Box::new(value)),
            Self::Float(value) => value,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum NumericStmt {
    SetInt(usize, IntExpr),
    SetFloat(usize, FloatExpr),
    EvalInt(IntExpr),
    EvalFloat(FloatExpr),
    CreateIntList(usize),
    AddIntList(usize, IntExpr),
    SortIntList(usize),
    ForRange {
        variable: usize,
        start: IntExpr,
        end: IntExpr,
        step: Option<IntExpr>,
        body: Vec<Self>,
        span: Span,
    },
    Return(Option<NumberExpr>),
}

#[derive(Clone, Debug)]
pub(crate) struct NumericFunction {
    pub(crate) parameters: Vec<NumericParameter>,
    pub(crate) int_locals: usize,
    pub(crate) float_locals: usize,
    pub(crate) int_list_locals: usize,
    pub(crate) statements: Vec<NumericStmt>,
}

#[derive(Clone, Copy)]
struct Local {
    ty: NumericType,
    slot: usize,
}

pub(crate) fn collect_global_types(program: &Program) -> HashMap<String, NumericType> {
    let mut globals = HashMap::from([
        ("PI".to_owned(), NumericType::Float),
        ("TAU".to_owned(), NumericType::Float),
        ("E".to_owned(), NumericType::Float),
        ("PHI".to_owned(), NumericType::Float),
        ("INF".to_owned(), NumericType::Float),
        ("NAN".to_owned(), NumericType::Float),
    ]);
    for statement in &program.statements {
        let Stmt::Assign {
            target: AssignTarget::Name(name, _),
            value,
            op: AssignOp::Set,
            ..
        } = statement
        else {
            continue;
        };
        if let Some(ty) = expression_type(value, &globals) {
            globals.insert(name.clone(), ty);
        }
    }
    globals
}

fn expression_type(
    expression: &Expr,
    globals: &HashMap<String, NumericType>,
) -> Option<NumericType> {
    match expression {
        Expr::Int(..) => Some(NumericType::Int),
        Expr::Float(..) => Some(NumericType::Float),
        Expr::Name(name, _) => globals.get(name).copied(),
        Expr::Unary {
            op: UnaryOp::Negate | UnaryOp::Positive,
            value,
            ..
        } => expression_type(value, globals),
        Expr::Binary {
            left, op, right, ..
        } => {
            let left = expression_type(left, globals)?;
            let right = expression_type(right, globals)?;
            match op {
                BinaryOp::Add
                | BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::IntegerDivide
                | BinaryOp::Remainder => {
                    Some(if left == NumericType::Int && right == NumericType::Int {
                        NumericType::Int
                    } else {
                        NumericType::Float
                    })
                }
                BinaryOp::Divide | BinaryOp::Power => Some(NumericType::Float),
                _ => None,
            }
        }
        Expr::Call { callee, args, .. }
            if args.len() == 1
                && matches!(callee.as_ref(), Expr::Name(name, _) if MathOp::from_name(name).is_some()) =>
        {
            expression_type(&args[0], globals).map(|_| NumericType::Float)
        }
        _ => None,
    }
}

pub(crate) fn compile_function(
    parameters: &[Param],
    body: &Block,
    globals: &HashMap<String, NumericType>,
) -> Option<NumericFunction> {
    let mut compiler = Compiler {
        globals,
        scopes: vec![HashMap::new()],
        int_locals: 0,
        float_locals: 0,
        int_list_locals: 0,
    };
    let mut compiled_parameters = Vec::with_capacity(parameters.len());
    for parameter in parameters {
        let ty = match parameter.ty {
            Some(TypeRef::Int) => NumericType::Int,
            Some(TypeRef::Float) => NumericType::Float,
            _ => return None,
        };
        let local = compiler.declare(&parameter.name, ty)?;
        compiled_parameters.push(match local.ty {
            NumericType::Int => NumericParameter::Int(local.slot),
            NumericType::Float => NumericParameter::Float(local.slot),
            NumericType::IntList => return None,
        });
    }
    let statements = compiler.statements(&body.statements, true)?;
    Some(NumericFunction {
        parameters: compiled_parameters,
        int_locals: compiler.int_locals,
        float_locals: compiler.float_locals,
        int_list_locals: compiler.int_list_locals,
        statements,
    })
}

struct Compiler<'a> {
    globals: &'a HashMap<String, NumericType>,
    scopes: Vec<HashMap<String, Local>>,
    int_locals: usize,
    float_locals: usize,
    int_list_locals: usize,
}

impl Compiler<'_> {
    fn statements(&mut self, statements: &[Stmt], allow_new: bool) -> Option<Vec<NumericStmt>> {
        statements
            .iter()
            .map(|statement| self.statement(statement, allow_new))
            .collect()
    }

    fn statement(&mut self, statement: &Stmt, allow_new: bool) -> Option<NumericStmt> {
        match statement {
            Stmt::Assign {
                target: AssignTarget::Name(name, _),
                ty: None,
                value,
                constant: false,
                op,
                ..
            } => {
                if matches!(op, AssignOp::Set)
                    && matches!(value, Expr::List(items, _) if items.is_empty())
                {
                    let local = if let Some(local) = self.local(name) {
                        local
                    } else {
                        if !allow_new || self.globals.contains_key(name) {
                            return None;
                        }
                        self.declare(name, NumericType::IntList)?
                    };
                    return (local.ty == NumericType::IntList)
                        .then_some(NumericStmt::CreateIntList(local.slot));
                }
                let right = self.expression(value)?;
                let local = if let Some(local) = self.local(name) {
                    local
                } else {
                    if !allow_new || self.globals.contains_key(name) || !matches!(op, AssignOp::Set)
                    {
                        return None;
                    }
                    self.declare(name, right.ty())?
                };
                match (local.ty, op, right) {
                    (NumericType::Int, AssignOp::Set, NumberExpr::Int(right)) => {
                        Some(NumericStmt::SetInt(local.slot, right))
                    }
                    (NumericType::Int, operation, NumberExpr::Int(right)) => {
                        let left = Box::new(IntExpr::Local(local.slot));
                        let right = Box::new(right);
                        let span = statement.span();
                        let expression = match operation {
                            AssignOp::Add => IntExpr::Add(left, right, span),
                            AssignOp::Subtract => IntExpr::Subtract(left, right, span),
                            AssignOp::Multiply => IntExpr::Multiply(left, right, span),
                            AssignOp::Divide | AssignOp::Set => return None,
                        };
                        Some(NumericStmt::SetInt(local.slot, expression))
                    }
                    (NumericType::Float, AssignOp::Set, right) => {
                        Some(NumericStmt::SetFloat(local.slot, right.into_float()))
                    }
                    (NumericType::Float, operation, right) => {
                        let left = Box::new(FloatExpr::Local(local.slot));
                        let right = Box::new(right.into_float());
                        let expression = match operation {
                            AssignOp::Add => FloatExpr::Add(left, right),
                            AssignOp::Subtract => FloatExpr::Subtract(left, right),
                            AssignOp::Multiply => FloatExpr::Multiply(left, right),
                            AssignOp::Divide => FloatExpr::Divide(left, right, statement.span()),
                            AssignOp::Set => return None,
                        };
                        Some(NumericStmt::SetFloat(local.slot, expression))
                    }
                    _ => None,
                }
            }
            Stmt::Expr(Expr::MemberCall {
                object, name, args, ..
            }) => {
                let Expr::Name(object, _) = object.as_ref() else {
                    return None;
                };
                let Some(Local {
                    ty: NumericType::IntList,
                    slot,
                }) = self.local(object)
                else {
                    return None;
                };
                match name.as_str() {
                    "add" if args.len() == 1 => {
                        let NumberExpr::Int(value) = self.expression(&args[0])? else {
                            return None;
                        };
                        Some(NumericStmt::AddIntList(slot, value))
                    }
                    "sort" if args.is_empty() => Some(NumericStmt::SortIntList(slot)),
                    _ => None,
                }
            }
            Stmt::Loop {
                kind:
                    LoopKind::For {
                        name,
                        iterable: Expr::Range { start, end, .. },
                        step,
                    },
                body,
                span,
            } => {
                let NumberExpr::Int(start) = self.expression(start)? else {
                    return None;
                };
                let NumberExpr::Int(end) = self.expression(end)? else {
                    return None;
                };
                let step = if let Some(step) = step {
                    let NumberExpr::Int(step) = self.expression(step)? else {
                        return None;
                    };
                    Some(step)
                } else {
                    None
                };
                self.scopes.push(HashMap::new());
                let variable = self.declare(name, NumericType::Int)?.slot;
                // Locals first assigned in a loop body are allocated once in the
                // numeric frame and overwritten on every iteration, matching the
                // evaluator's loop-scope lifetime without per-iteration maps.
                let compiled_body = self.statements(&body.statements, true);
                self.scopes.pop();
                Some(NumericStmt::ForRange {
                    variable,
                    start,
                    end,
                    step,
                    body: compiled_body?,
                    span: *span,
                })
            }
            Stmt::Return(value, _) => Some(NumericStmt::Return(
                value
                    .as_ref()
                    .map(|value| self.expression(value))
                    .transpose()?,
            )),
            Stmt::Expr(expression) => match self.expression(expression)? {
                NumberExpr::Int(expression) => Some(NumericStmt::EvalInt(expression)),
                NumberExpr::Float(expression) => Some(NumericStmt::EvalFloat(expression)),
            },
            _ => None,
        }
    }

    fn expression(&self, expression: &Expr) -> Option<NumberExpr> {
        match expression {
            Expr::Int(value, _) => Some(NumberExpr::Int(IntExpr::Literal(*value))),
            Expr::Float(value, _) => Some(NumberExpr::Float(FloatExpr::Literal(*value))),
            Expr::Name(name, span) => match self.local(name) {
                Some(Local {
                    ty: NumericType::Int,
                    slot,
                }) => Some(NumberExpr::Int(IntExpr::Local(slot))),
                Some(Local {
                    ty: NumericType::Float,
                    slot,
                }) => Some(NumberExpr::Float(FloatExpr::Local(slot))),
                Some(Local {
                    ty: NumericType::IntList,
                    ..
                }) => None,
                None => match self.globals.get(name)? {
                    NumericType::Int => Some(NumberExpr::Int(IntExpr::Global(name.clone(), *span))),
                    NumericType::Float => {
                        Some(NumberExpr::Float(FloatExpr::Global(name.clone(), *span)))
                    }
                    NumericType::IntList => None,
                },
            },
            Expr::Unary { op, value, span } => {
                let value = self.expression(value)?;
                match (op, value) {
                    (UnaryOp::Positive, value) => Some(value),
                    (UnaryOp::Negate, NumberExpr::Int(value)) => {
                        Some(NumberExpr::Int(IntExpr::Negate(Box::new(value), *span)))
                    }
                    (UnaryOp::Negate, NumberExpr::Float(value)) => {
                        Some(NumberExpr::Float(FloatExpr::Negate(Box::new(value))))
                    }
                    (UnaryOp::Not, _) => None,
                }
            }
            Expr::Binary {
                left,
                op,
                right,
                span,
            } => self.binary(left, *op, right, *span),
            Expr::Call { callee, args, span } if args.len() == 1 => {
                let Expr::Name(name, _) = callee.as_ref() else {
                    return None;
                };
                let operation = MathOp::from_name(name)?;
                let value = self.expression(&args[0])?.into_float();
                Some(NumberExpr::Float(FloatExpr::Math(
                    operation,
                    Box::new(value),
                    *span,
                )))
            }
            Expr::MemberCall {
                object,
                name,
                args,
                span,
            } if args.is_empty() && name == "len" => {
                let Expr::Name(name, _) = object.as_ref() else {
                    return None;
                };
                let Local {
                    ty: NumericType::IntList,
                    slot,
                } = self.local(name)?
                else {
                    return None;
                };
                let _ = span;
                Some(NumberExpr::Int(IntExpr::ListLength(slot)))
            }
            Expr::Index {
                object,
                index,
                span,
            } => {
                let Expr::Name(name, _) = object.as_ref() else {
                    return None;
                };
                let Local {
                    ty: NumericType::IntList,
                    slot,
                } = self.local(name)?
                else {
                    return None;
                };
                let NumberExpr::Int(index) = self.expression(index)? else {
                    return None;
                };
                Some(NumberExpr::Int(IntExpr::ListIndex(
                    slot,
                    Box::new(index),
                    *span,
                )))
            }
            _ => None,
        }
    }

    fn binary(
        &self,
        left: &Expr,
        operation: BinaryOp,
        right: &Expr,
        span: Span,
    ) -> Option<NumberExpr> {
        let left = self.expression(left)?;
        let right = self.expression(right)?;
        if matches!(left, NumberExpr::Int(_)) && matches!(right, NumberExpr::Int(_)) {
            let NumberExpr::Int(left) = left else {
                unreachable!()
            };
            let NumberExpr::Int(right) = right else {
                unreachable!()
            };
            let left = Box::new(left);
            let right = Box::new(right);
            return Some(match operation {
                BinaryOp::Add => NumberExpr::Int(IntExpr::Add(left, right, span)),
                BinaryOp::Subtract => NumberExpr::Int(IntExpr::Subtract(left, right, span)),
                BinaryOp::Multiply => NumberExpr::Int(IntExpr::Multiply(left, right, span)),
                BinaryOp::IntegerDivide => {
                    NumberExpr::Int(IntExpr::IntegerDivide(left, right, span))
                }
                BinaryOp::Remainder => NumberExpr::Int(IntExpr::Remainder(left, right, span)),
                BinaryOp::Divide => NumberExpr::Float(FloatExpr::Divide(
                    Box::new(FloatExpr::FromInt(left)),
                    Box::new(FloatExpr::FromInt(right)),
                    span,
                )),
                BinaryOp::Power => NumberExpr::Float(FloatExpr::Power(
                    Box::new(FloatExpr::FromInt(left)),
                    Box::new(FloatExpr::FromInt(right)),
                )),
                _ => return None,
            });
        }

        let left = Box::new(left.into_float());
        let right = Box::new(right.into_float());
        Some(NumberExpr::Float(match operation {
            BinaryOp::Add => FloatExpr::Add(left, right),
            BinaryOp::Subtract => FloatExpr::Subtract(left, right),
            BinaryOp::Multiply => FloatExpr::Multiply(left, right),
            BinaryOp::Divide => FloatExpr::Divide(left, right, span),
            BinaryOp::IntegerDivide => FloatExpr::IntegerDivide(left, right, span),
            BinaryOp::Remainder => FloatExpr::Remainder(left, right, span),
            BinaryOp::Power => FloatExpr::Power(left, right),
            _ => return None,
        }))
    }

    fn local(&self, name: &str) -> Option<Local> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    fn declare(&mut self, name: &str, ty: NumericType) -> Option<Local> {
        let scope = self.scopes.last_mut()?;
        if scope.contains_key(name) {
            return None;
        }
        let slot = match ty {
            NumericType::Int => {
                let slot = self.int_locals;
                self.int_locals += 1;
                slot
            }
            NumericType::Float => {
                let slot = self.float_locals;
                self.float_locals += 1;
                slot
            }
            NumericType::IntList => {
                let slot = self.int_list_locals;
                self.int_list_locals += 1;
                slot
            }
        };
        let local = Local { ty, slot };
        scope.insert(name.to_owned(), local);
        Some(local)
    }
}

trait OptionTranspose<T> {
    fn transpose(self) -> Option<Option<T>>;
}

impl<T> OptionTranspose<T> for Option<Option<T>> {
    fn transpose(self) -> Option<Option<T>> {
        match self {
            Some(Some(value)) => Some(Some(value)),
            Some(None) => None,
            None => Some(None),
        }
    }
}
