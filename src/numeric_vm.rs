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
    Bool,
    IntList,
    FloatList,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum NumericParameter {
    Int(usize),
    Float(usize),
    Bool(usize),
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

#[derive(Clone, Copy, Debug)]
pub(crate) enum CompareOp {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

impl CompareOp {
    fn from_binary(operation: BinaryOp) -> Option<Self> {
        Some(match operation {
            BinaryOp::Equal => Self::Equal,
            BinaryOp::NotEqual => Self::NotEqual,
            BinaryOp::Less => Self::Less,
            BinaryOp::LessEqual => Self::LessEqual,
            BinaryOp::Greater => Self::Greater,
            BinaryOp::GreaterEqual => Self::GreaterEqual,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum IntListOp {
    Sum,
    Product,
    Min,
    Max,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum FloatListOp {
    Sum,
    Product,
    Min,
    Max,
    Mean,
    Median,
    Mode,
    Variance,
    Std,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum NumericListRef {
    Int(usize),
    Float(usize),
}

#[derive(Clone, Debug)]
pub(crate) struct NumericCall {
    pub(crate) name: String,
    pub(crate) args: Vec<NumericValueExpr>,
    pub(crate) builtin: bool,
    pub(crate) span: Span,
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
    ListLength(NumericListRef),
    ListAggregate(usize, IntListOp, Span),
    Call(NumericCall),
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
    Power(Box<Self>, Box<Self>, Span),
    Math(MathOp, Box<Self>, Span),
    ListIndex(usize, Box<IntExpr>, Span),
    ListAggregate(NumericListRef, FloatListOp, Span),
    Call(NumericCall),
}

#[derive(Clone, Debug)]
pub(crate) enum NumberExpr {
    Int(IntExpr),
    Float(FloatExpr),
}

impl NumberExpr {
    pub(crate) fn into_float(self) -> FloatExpr {
        match self {
            Self::Int(value) => FloatExpr::FromInt(Box::new(value)),
            Self::Float(value) => value,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum BoolExpr {
    Literal(bool),
    Local(usize),
    Global(String, Span),
    Not(Box<Self>),
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    IntTruthy(Box<IntExpr>),
    FloatTruthy(Box<FloatExpr>),
    NumberCompare(Box<NumberExpr>, CompareOp, Box<NumberExpr>, Span),
    BoolCompare(Box<Self>, CompareOp, Box<Self>),
    Call(NumericCall),
}

#[derive(Clone, Debug)]
pub(crate) enum NumericValueExpr {
    Int(IntExpr),
    Float(FloatExpr),
    Bool(BoolExpr),
}

impl NumericValueExpr {
    fn ty(&self) -> NumericType {
        match self {
            Self::Int(_) => NumericType::Int,
            Self::Float(_) => NumericType::Float,
            Self::Bool(_) => NumericType::Bool,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum NumericStmt {
    SetInt(usize, IntExpr),
    SetFloat(usize, FloatExpr),
    SetBool(usize, BoolExpr),
    EvalInt(IntExpr),
    EvalFloat(FloatExpr),
    EvalBool(BoolExpr),
    SetIntList(usize, Vec<IntExpr>),
    SetFloatList(usize, Vec<FloatExpr>),
    AddIntList(usize, Vec<IntExpr>),
    AddFloatList(usize, Vec<FloatExpr>),
    SetIntListIndex(usize, IntExpr, IntExpr, Span),
    SetFloatListIndex(usize, IntExpr, FloatExpr, Span),
    SortIntList(usize),
    SortFloatList(usize),
    ReverseIntList(usize),
    ReverseFloatList(usize),
    ClearIntList(usize),
    ClearFloatList(usize),
    If {
        condition: BoolExpr,
        then_body: Vec<Self>,
        else_body: Vec<Self>,
    },
    Loop {
        condition: Option<BoolExpr>,
        body: Vec<Self>,
        span: Span,
    },
    ForRange {
        variable: usize,
        start: IntExpr,
        end: IntExpr,
        step: Option<IntExpr>,
        body: Vec<Self>,
        span: Span,
    },
    ForIntList {
        variable: usize,
        list: usize,
        body: Vec<Self>,
        span: Span,
    },
    ForFloatList {
        variable: usize,
        list: usize,
        body: Vec<Self>,
        span: Span,
    },
    Break,
    Continue,
    Return(Option<NumericValueExpr>),
}

#[derive(Clone, Debug)]
pub(crate) struct NumericFunction {
    pub(crate) parameters: Vec<NumericParameter>,
    pub(crate) int_locals: usize,
    pub(crate) float_locals: usize,
    pub(crate) bool_locals: usize,
    pub(crate) int_list_locals: usize,
    pub(crate) float_list_locals: usize,
    pub(crate) statements: Vec<NumericStmt>,
}

#[derive(Default)]
pub(crate) struct CompileEnvironment {
    globals: HashMap<String, NumericType>,
    functions: HashMap<String, Option<NumericType>>,
}

#[derive(Clone, Copy)]
struct Local {
    ty: NumericType,
    slot: usize,
    constant: bool,
}

pub(crate) fn collect_environment(program: &Program) -> CompileEnvironment {
    let mut environment = CompileEnvironment {
        globals: HashMap::from([
            ("PI".to_owned(), NumericType::Float),
            ("TAU".to_owned(), NumericType::Float),
            ("E".to_owned(), NumericType::Float),
            ("PHI".to_owned(), NumericType::Float),
            ("INF".to_owned(), NumericType::Float),
            ("NAN".to_owned(), NumericType::Float),
        ]),
        functions: HashMap::new(),
    };

    for statement in &program.statements {
        if let Stmt::Function {
            name, return_type, ..
        } = statement
        {
            environment
                .functions
                .insert(name.clone(), return_type.as_ref().and_then(scalar_type_ref));
        }
    }

    for statement in &program.statements {
        let Stmt::Assign {
            target: AssignTarget::Name(name, _),
            ty,
            value,
            op: AssignOp::Set,
            ..
        } = statement
        else {
            continue;
        };
        let inferred = ty
            .as_ref()
            .and_then(scalar_type_ref)
            .or_else(|| expression_type(value, &environment));
        if let Some(ty) = inferred {
            environment.globals.insert(name.clone(), ty);
        }
    }
    environment
}

fn scalar_type_ref(ty: &TypeRef) -> Option<NumericType> {
    match ty {
        TypeRef::Int => Some(NumericType::Int),
        TypeRef::Float => Some(NumericType::Float),
        TypeRef::Bool => Some(NumericType::Bool),
        _ => None,
    }
}

fn local_type_ref(ty: &TypeRef) -> Option<NumericType> {
    match ty {
        TypeRef::List(Some(item)) if matches!(item.as_ref(), TypeRef::Int) => {
            Some(NumericType::IntList)
        }
        TypeRef::List(Some(item)) if matches!(item.as_ref(), TypeRef::Float) => {
            Some(NumericType::FloatList)
        }
        _ => scalar_type_ref(ty),
    }
}

fn expression_type(expression: &Expr, environment: &CompileEnvironment) -> Option<NumericType> {
    match expression {
        Expr::Bool(..) => Some(NumericType::Bool),
        Expr::Int(..) => Some(NumericType::Int),
        Expr::Float(..) => Some(NumericType::Float),
        Expr::Name(name, _) => environment.globals.get(name).copied(),
        Expr::Unary {
            op: UnaryOp::Not, ..
        } => Some(NumericType::Bool),
        Expr::Unary {
            op: UnaryOp::Negate | UnaryOp::Positive,
            value,
            ..
        } => expression_type(value, environment),
        Expr::Binary {
            left, op, right, ..
        } => match op {
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual
            | BinaryOp::And
            | BinaryOp::Or => Some(NumericType::Bool),
            BinaryOp::Divide | BinaryOp::Power => Some(NumericType::Float),
            BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::IntegerDivide
            | BinaryOp::Remainder => {
                let left = expression_type(left, environment)?;
                let right = expression_type(right, environment)?;
                if left == NumericType::Int && right == NumericType::Int {
                    Some(NumericType::Int)
                } else if matches!(left, NumericType::Int | NumericType::Float)
                    && matches!(right, NumericType::Int | NumericType::Float)
                {
                    Some(NumericType::Float)
                } else {
                    None
                }
            }
            BinaryOp::In => None,
        },
        Expr::Call { callee, args, .. } => {
            let Expr::Name(name, _) = callee.as_ref() else {
                return None;
            };
            if let Some(return_type) = environment.functions.get(name).copied().flatten() {
                return Some(return_type);
            }
            if environment.functions.contains_key(name) {
                return None;
            }
            if args.len() == 1 && MathOp::from_name(name).is_some() {
                return Some(NumericType::Float);
            }
            builtin_scalar_type(name, args, &|value| expression_type(value, environment))
        }
        _ => None,
    }
}

pub(crate) fn compile_function(
    parameters: &[Param],
    body: &Block,
    environment: &CompileEnvironment,
) -> Option<NumericFunction> {
    let mut compiler = Compiler {
        environment,
        scopes: vec![HashMap::new()],
        int_locals: 0,
        float_locals: 0,
        bool_locals: 0,
        int_list_locals: 0,
        float_list_locals: 0,
        loop_depth: 0,
    };
    let mut compiled_parameters = Vec::with_capacity(parameters.len());
    for parameter in parameters {
        let ty = scalar_type_ref(parameter.ty.as_ref()?)?;
        let local = compiler.declare(&parameter.name, ty)?;
        compiled_parameters.push(match local.ty {
            NumericType::Int => NumericParameter::Int(local.slot),
            NumericType::Float => NumericParameter::Float(local.slot),
            NumericType::Bool => NumericParameter::Bool(local.slot),
            NumericType::IntList | NumericType::FloatList => return None,
        });
    }
    let statements = compiler.statements(&body.statements, true)?;
    Some(NumericFunction {
        parameters: compiled_parameters,
        int_locals: compiler.int_locals,
        float_locals: compiler.float_locals,
        bool_locals: compiler.bool_locals,
        int_list_locals: compiler.int_list_locals,
        float_list_locals: compiler.float_list_locals,
        statements,
    })
}

struct Compiler<'a> {
    environment: &'a CompileEnvironment,
    scopes: Vec<HashMap<String, Local>>,
    int_locals: usize,
    float_locals: usize,
    bool_locals: usize,
    int_list_locals: usize,
    float_list_locals: usize,
    loop_depth: usize,
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
                ty,
                value,
                constant: true,
                op: AssignOp::Set,
                ..
            } if !matches!(value, Expr::List(..)) => {
                if !allow_new
                    || self.local(name).is_some()
                    || self.environment.globals.contains_key(name)
                {
                    return None;
                }
                let right = self.value_expression(value)?;
                let declared = ty.as_ref().map(local_type_ref).transpose()?;
                if declared.is_some_and(|declared| declared != right.ty()) {
                    return None;
                }
                let local =
                    self.declare_with_constant(name, declared.unwrap_or_else(|| right.ty()), true)?;
                self.scalar_assignment(local, right, AssignOp::Set, statement.span())
            }
            Stmt::Assign {
                target: AssignTarget::Name(name, _),
                ty,
                value: value @ Expr::List(..),
                constant: true,
                op: AssignOp::Set,
                ..
            } => self.list_assignment(name, ty.as_ref(), value, AssignOp::Set, allow_new, true),
            Stmt::Assign {
                target: AssignTarget::Name(name, _),
                ty,
                value,
                constant: false,
                op,
                ..
            } => {
                if matches!(value, Expr::List(..)) {
                    return self.list_assignment(name, ty.as_ref(), value, *op, allow_new, false);
                }
                let right = self.value_expression(value)?;
                let declared = ty.as_ref().map(local_type_ref).transpose()?;
                if declared.is_some_and(|declared| declared != right.ty()) {
                    return None;
                }
                let local = if let Some(local) = self.local(name) {
                    if ty.is_some() || local.constant {
                        return None;
                    }
                    local
                } else {
                    if !allow_new
                        || self.environment.globals.contains_key(name)
                        || !matches!(op, AssignOp::Set)
                    {
                        return None;
                    }
                    self.declare(name, declared.unwrap_or_else(|| right.ty()))?
                };
                self.scalar_assignment(local, right, *op, statement.span())
            }
            Stmt::Assign {
                target: AssignTarget::Index(object, index, target_span),
                ty: None,
                value,
                constant: false,
                op: AssignOp::Set,
                ..
            } => self.list_index_assignment(object, index, value, *target_span),
            Stmt::Expr(Expr::MemberCall {
                object, name, args, ..
            }) => self.list_mutation(object, name, args),
            Stmt::If {
                condition,
                then_block,
                else_branch,
                ..
            } => {
                let condition = self.condition_expression(condition)?;
                self.scopes.push(HashMap::new());
                let then_body = self.statements(&then_block.statements, true);
                self.scopes.pop();
                let else_body = if let Some(branch) = else_branch {
                    vec![self.statement(branch, true)?]
                } else {
                    vec![]
                };
                Some(NumericStmt::If {
                    condition,
                    then_body: then_body?,
                    else_body,
                })
            }
            Stmt::Loop { kind, body, span } => self.loop_statement(kind, body, *span),
            Stmt::Return(value, _) => Some(NumericStmt::Return(
                value
                    .as_ref()
                    .map(|value| self.value_expression(value))
                    .transpose()?,
            )),
            Stmt::Break(_) if self.loop_depth > 0 => Some(NumericStmt::Break),
            Stmt::Continue(_) if self.loop_depth > 0 => Some(NumericStmt::Continue),
            Stmt::Expr(expression) => match self.value_expression(expression)? {
                NumericValueExpr::Int(expression) => Some(NumericStmt::EvalInt(expression)),
                NumericValueExpr::Float(expression) => Some(NumericStmt::EvalFloat(expression)),
                NumericValueExpr::Bool(expression) => Some(NumericStmt::EvalBool(expression)),
            },
            _ => None,
        }
    }

    fn scalar_assignment(
        &self,
        local: Local,
        right: NumericValueExpr,
        operation: AssignOp,
        span: Span,
    ) -> Option<NumericStmt> {
        match (local.ty, operation, right) {
            (NumericType::Int, AssignOp::Set, NumericValueExpr::Int(right)) => {
                Some(NumericStmt::SetInt(local.slot, right))
            }
            (NumericType::Int, operation, NumericValueExpr::Int(right)) => {
                let left = Box::new(IntExpr::Local(local.slot));
                let right = Box::new(right);
                let expression = match operation {
                    AssignOp::Add => IntExpr::Add(left, right, span),
                    AssignOp::Subtract => IntExpr::Subtract(left, right, span),
                    AssignOp::Multiply => IntExpr::Multiply(left, right, span),
                    AssignOp::Divide | AssignOp::Set => return None,
                };
                Some(NumericStmt::SetInt(local.slot, expression))
            }
            (NumericType::Float, AssignOp::Set, NumericValueExpr::Float(right)) => {
                Some(NumericStmt::SetFloat(local.slot, right))
            }
            (NumericType::Float, operation, right)
                if matches!(right, NumericValueExpr::Int(_) | NumericValueExpr::Float(_)) =>
            {
                let right = match right {
                    NumericValueExpr::Int(value) => NumberExpr::Int(value),
                    NumericValueExpr::Float(value) => NumberExpr::Float(value),
                    NumericValueExpr::Bool(_) => unreachable!(),
                };
                let left = Box::new(FloatExpr::Local(local.slot));
                let right = Box::new(right.into_float());
                let expression = match operation {
                    AssignOp::Add => FloatExpr::Add(left, right),
                    AssignOp::Subtract => FloatExpr::Subtract(left, right),
                    AssignOp::Multiply => FloatExpr::Multiply(left, right),
                    AssignOp::Divide => FloatExpr::Divide(left, right, span),
                    AssignOp::Set => return None,
                };
                Some(NumericStmt::SetFloat(local.slot, expression))
            }
            (NumericType::Bool, AssignOp::Set, NumericValueExpr::Bool(right)) => {
                Some(NumericStmt::SetBool(local.slot, right))
            }
            _ => None,
        }
    }

    fn list_assignment(
        &mut self,
        name: &str,
        annotation: Option<&TypeRef>,
        value: &Expr,
        operation: AssignOp,
        allow_new: bool,
        constant: bool,
    ) -> Option<NumericStmt> {
        if !matches!(operation, AssignOp::Set) {
            return None;
        }
        let Expr::List(items, _) = value else {
            return None;
        };
        let declared = annotation.map(local_type_ref).transpose()?;
        if declared.is_some_and(|ty| !matches!(ty, NumericType::IntList | NumericType::FloatList)) {
            return None;
        }
        let existing = self.local(name);
        if existing.is_some() && (annotation.is_some() || constant) {
            return None;
        }
        let inferred = if items.is_empty() {
            declared
                .or(existing.map(|local| local.ty))
                .unwrap_or(NumericType::IntList)
        } else if items
            .iter()
            .all(|item| matches!(self.value_type(item), Some(NumericType::Int)))
        {
            NumericType::IntList
        } else if items
            .iter()
            .all(|item| matches!(self.value_type(item), Some(NumericType::Float)))
        {
            NumericType::FloatList
        } else {
            return None;
        };
        if declared.is_some_and(|declared| declared != inferred) {
            return None;
        }
        let local = if let Some(local) = existing {
            if local.ty != inferred || local.constant {
                return None;
            }
            local
        } else {
            if !allow_new || self.environment.globals.contains_key(name) {
                return None;
            }
            self.declare_with_constant(name, inferred, constant)?
        };
        match inferred {
            NumericType::IntList => Some(NumericStmt::SetIntList(
                local.slot,
                items
                    .iter()
                    .map(|item| match self.value_expression(item)? {
                        NumericValueExpr::Int(value) => Some(value),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>()?,
            )),
            NumericType::FloatList => Some(NumericStmt::SetFloatList(
                local.slot,
                items
                    .iter()
                    .map(|item| match self.value_expression(item)? {
                        NumericValueExpr::Float(value) => Some(value),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>()?,
            )),
            _ => None,
        }
    }

    fn list_index_assignment(
        &self,
        object: &Expr,
        index: &Expr,
        value: &Expr,
        span: Span,
    ) -> Option<NumericStmt> {
        let Expr::Name(name, _) = object else {
            return None;
        };
        let local = self.local(name)?;
        if local.constant {
            return None;
        }
        let NumericValueExpr::Int(index) = self.value_expression(index)? else {
            return None;
        };
        match (local.ty, self.value_expression(value)?) {
            (NumericType::IntList, NumericValueExpr::Int(value)) => {
                Some(NumericStmt::SetIntListIndex(local.slot, index, value, span))
            }
            (NumericType::FloatList, NumericValueExpr::Float(value)) => Some(
                NumericStmt::SetFloatListIndex(local.slot, index, value, span),
            ),
            _ => None,
        }
    }

    fn list_mutation(&self, object: &Expr, name: &str, args: &[Expr]) -> Option<NumericStmt> {
        let Expr::Name(object, _) = object else {
            return None;
        };
        let local = self.local(object)?;
        if local.constant {
            return None;
        }
        match (local.ty, name) {
            (NumericType::IntList, "add") if !args.is_empty() => Some(NumericStmt::AddIntList(
                local.slot,
                args.iter()
                    .map(|argument| match self.value_expression(argument)? {
                        NumericValueExpr::Int(value) => Some(value),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>()?,
            )),
            (NumericType::FloatList, "add") if !args.is_empty() => Some(NumericStmt::AddFloatList(
                local.slot,
                args.iter()
                    .map(|argument| match self.value_expression(argument)? {
                        NumericValueExpr::Float(value) => Some(value),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>()?,
            )),
            (NumericType::IntList, "sort") if args.is_empty() => {
                Some(NumericStmt::SortIntList(local.slot))
            }
            (NumericType::FloatList, "sort") if args.is_empty() => {
                Some(NumericStmt::SortFloatList(local.slot))
            }
            (NumericType::IntList, "reverse") if args.is_empty() => {
                Some(NumericStmt::ReverseIntList(local.slot))
            }
            (NumericType::FloatList, "reverse") if args.is_empty() => {
                Some(NumericStmt::ReverseFloatList(local.slot))
            }
            (NumericType::IntList, "clear") if args.is_empty() => {
                Some(NumericStmt::ClearIntList(local.slot))
            }
            (NumericType::FloatList, "clear") if args.is_empty() => {
                Some(NumericStmt::ClearFloatList(local.slot))
            }
            _ => None,
        }
    }

    fn loop_statement(&mut self, kind: &LoopKind, body: &Block, span: Span) -> Option<NumericStmt> {
        match kind {
            LoopKind::Forever | LoopKind::While(_) => {
                let condition = match kind {
                    LoopKind::While(condition) => Some(self.condition_expression(condition)?),
                    LoopKind::Forever => None,
                    LoopKind::For { .. } => unreachable!(),
                };
                self.scopes.push(HashMap::new());
                self.loop_depth += 1;
                let compiled_body = self.statements(&body.statements, true);
                self.loop_depth -= 1;
                self.scopes.pop();
                Some(NumericStmt::Loop {
                    condition,
                    body: compiled_body?,
                    span,
                })
            }
            LoopKind::For {
                name,
                iterable: Expr::Range { start, end, .. },
                step,
            } => {
                let NumericValueExpr::Int(start) = self.value_expression(start)? else {
                    return None;
                };
                let NumericValueExpr::Int(end) = self.value_expression(end)? else {
                    return None;
                };
                let step = if let Some(step) = step {
                    let NumericValueExpr::Int(step) = self.value_expression(step)? else {
                        return None;
                    };
                    Some(step)
                } else {
                    None
                };
                self.scopes.push(HashMap::new());
                let variable = self.declare(name, NumericType::Int)?.slot;
                self.loop_depth += 1;
                let compiled_body = self.statements(&body.statements, true);
                self.loop_depth -= 1;
                self.scopes.pop();
                Some(NumericStmt::ForRange {
                    variable,
                    start,
                    end,
                    step,
                    body: compiled_body?,
                    span,
                })
            }
            LoopKind::For {
                name,
                iterable: Expr::Name(list_name, _),
                step: None,
            } => {
                let list = self.local(list_name)?;
                let item_type = match list.ty {
                    NumericType::IntList => NumericType::Int,
                    NumericType::FloatList => NumericType::Float,
                    _ => return None,
                };
                self.scopes.push(HashMap::new());
                let variable = self.declare(name, item_type)?.slot;
                self.loop_depth += 1;
                let compiled_body = self.statements(&body.statements, true);
                self.loop_depth -= 1;
                self.scopes.pop();
                match list.ty {
                    NumericType::IntList => Some(NumericStmt::ForIntList {
                        variable,
                        list: list.slot,
                        body: compiled_body?,
                        span,
                    }),
                    NumericType::FloatList => Some(NumericStmt::ForFloatList {
                        variable,
                        list: list.slot,
                        body: compiled_body?,
                        span,
                    }),
                    _ => None,
                }
            }
            LoopKind::For { .. } => None,
        }
    }

    fn value_expression(&self, expression: &Expr) -> Option<NumericValueExpr> {
        match self.value_type(expression)? {
            NumericType::Int => self
                .number_expression(expression)
                .and_then(|value| match value {
                    NumberExpr::Int(value) => Some(NumericValueExpr::Int(value)),
                    NumberExpr::Float(_) => None,
                }),
            NumericType::Float => {
                self.number_expression(expression)
                    .and_then(|value| match value {
                        NumberExpr::Float(value) => Some(NumericValueExpr::Float(value)),
                        NumberExpr::Int(_) => None,
                    })
            }
            NumericType::Bool => self.bool_expression(expression).map(NumericValueExpr::Bool),
            NumericType::IntList | NumericType::FloatList => None,
        }
    }

    fn value_type(&self, expression: &Expr) -> Option<NumericType> {
        match expression {
            Expr::Bool(..) => Some(NumericType::Bool),
            Expr::Int(..) => Some(NumericType::Int),
            Expr::Float(..) => Some(NumericType::Float),
            Expr::Name(name, _) => self
                .local(name)
                .map(|local| local.ty)
                .or_else(|| self.environment.globals.get(name).copied()),
            Expr::Unary {
                op: UnaryOp::Not, ..
            } => Some(NumericType::Bool),
            Expr::Unary {
                op: UnaryOp::Negate | UnaryOp::Positive,
                value,
                ..
            } => self.value_type(value),
            Expr::Binary {
                left, op, right, ..
            } => match op {
                BinaryOp::Equal
                | BinaryOp::NotEqual
                | BinaryOp::Less
                | BinaryOp::LessEqual
                | BinaryOp::Greater
                | BinaryOp::GreaterEqual
                | BinaryOp::And
                | BinaryOp::Or => Some(NumericType::Bool),
                BinaryOp::Divide | BinaryOp::Power => Some(NumericType::Float),
                BinaryOp::Add
                | BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::IntegerDivide
                | BinaryOp::Remainder => {
                    let left = self.value_type(left)?;
                    let right = self.value_type(right)?;
                    if left == NumericType::Int && right == NumericType::Int {
                        Some(NumericType::Int)
                    } else if matches!(left, NumericType::Int | NumericType::Float)
                        && matches!(right, NumericType::Int | NumericType::Float)
                    {
                        Some(NumericType::Float)
                    } else {
                        None
                    }
                }
                BinaryOp::In => None,
            },
            Expr::Call { callee, args, .. } => {
                let Expr::Name(name, _) = callee.as_ref() else {
                    return None;
                };
                if let Some(return_type) = self.environment.functions.get(name).copied().flatten() {
                    return Some(return_type);
                }
                if self.environment.functions.contains_key(name) {
                    return None;
                }
                if args.len() == 1 && MathOp::from_name(name).is_some() {
                    return Some(NumericType::Float);
                }
                if args.len() == 1 {
                    if let Expr::Name(list_name, _) = &args[0] {
                        if let Some(list) = self.local(list_name) {
                            if matches!(list.ty, NumericType::IntList | NumericType::FloatList) {
                                return self.list_method_type(&args[0], name);
                            }
                        }
                    }
                }
                builtin_scalar_type(name, args, &|value| self.value_type(value))
            }
            Expr::MemberCall {
                object, name, args, ..
            } if args.is_empty() => self.list_method_type(object, name),
            Expr::Index { object, .. } => {
                let Expr::Name(name, _) = object.as_ref() else {
                    return None;
                };
                match self.local(name)?.ty {
                    NumericType::IntList => Some(NumericType::Int),
                    NumericType::FloatList => Some(NumericType::Float),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn number_expression(&self, expression: &Expr) -> Option<NumberExpr> {
        match expression {
            Expr::Int(value, _) => Some(NumberExpr::Int(IntExpr::Literal(*value))),
            Expr::Float(value, _) => Some(NumberExpr::Float(FloatExpr::Literal(*value))),
            Expr::Name(name, span) => match self.local(name) {
                Some(Local {
                    ty: NumericType::Int,
                    slot,
                    ..
                }) => Some(NumberExpr::Int(IntExpr::Local(slot))),
                Some(Local {
                    ty: NumericType::Float,
                    slot,
                    ..
                }) => Some(NumberExpr::Float(FloatExpr::Local(slot))),
                Some(_) => None,
                None => match self.environment.globals.get(name)? {
                    NumericType::Int => Some(NumberExpr::Int(IntExpr::Global(name.clone(), *span))),
                    NumericType::Float => {
                        Some(NumberExpr::Float(FloatExpr::Global(name.clone(), *span)))
                    }
                    _ => None,
                },
            },
            Expr::Unary { op, value, span } => {
                let value = self.number_expression(value)?;
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
            Expr::Call { callee, args, span } => {
                let Expr::Name(name, _) = callee.as_ref() else {
                    return None;
                };
                if !self.environment.functions.contains_key(name) && args.len() == 1 {
                    if let Some(operation) = MathOp::from_name(name) {
                        let value = self.number_expression(&args[0])?.into_float();
                        return Some(NumberExpr::Float(FloatExpr::Math(
                            operation,
                            Box::new(value),
                            *span,
                        )));
                    }
                }
                if let Some(aggregate) = self.global_list_aggregate(name, args, *span) {
                    return Some(aggregate);
                }
                let return_type = self.value_type(expression)?;
                let call = self.numeric_call(name, args, *span)?;
                match return_type {
                    NumericType::Int => Some(NumberExpr::Int(IntExpr::Call(call))),
                    NumericType::Float => Some(NumberExpr::Float(FloatExpr::Call(call))),
                    _ => None,
                }
            }
            Expr::MemberCall {
                object,
                name,
                args,
                span,
            } if args.is_empty() => self.list_method_expression(object, name, *span),
            Expr::Index {
                object,
                index,
                span,
            } => {
                let Expr::Name(name, _) = object.as_ref() else {
                    return None;
                };
                let NumericValueExpr::Int(index) = self.value_expression(index)? else {
                    return None;
                };
                let local = self.local(name)?;
                match local.ty {
                    NumericType::IntList => Some(NumberExpr::Int(IntExpr::ListIndex(
                        local.slot,
                        Box::new(index),
                        *span,
                    ))),
                    NumericType::FloatList => Some(NumberExpr::Float(FloatExpr::ListIndex(
                        local.slot,
                        Box::new(index),
                        *span,
                    ))),
                    _ => None,
                }
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
        let left = self.number_expression(left)?;
        let right = self.number_expression(right)?;
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
                    span,
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
            BinaryOp::Power => FloatExpr::Power(left, right, span),
            _ => return None,
        }))
    }

    fn bool_expression(&self, expression: &Expr) -> Option<BoolExpr> {
        match expression {
            Expr::Bool(value, _) => Some(BoolExpr::Literal(*value)),
            Expr::Name(name, span) => match self.local(name) {
                Some(Local {
                    ty: NumericType::Bool,
                    slot,
                    ..
                }) => Some(BoolExpr::Local(slot)),
                Some(_) => None,
                None if self.environment.globals.get(name) == Some(&NumericType::Bool) => {
                    Some(BoolExpr::Global(name.clone(), *span))
                }
                None => None,
            },
            Expr::Unary {
                op: UnaryOp::Not,
                value,
                ..
            } => Some(BoolExpr::Not(Box::new(self.condition_expression(value)?))),
            Expr::Binary {
                left,
                op: BinaryOp::And,
                right,
                ..
            } => Some(BoolExpr::And(
                Box::new(self.condition_expression(left)?),
                Box::new(self.condition_expression(right)?),
            )),
            Expr::Binary {
                left,
                op: BinaryOp::Or,
                right,
                ..
            } => Some(BoolExpr::Or(
                Box::new(self.condition_expression(left)?),
                Box::new(self.condition_expression(right)?),
            )),
            Expr::Binary {
                left,
                op,
                right,
                span,
            } => {
                let operation = CompareOp::from_binary(*op)?;
                match (self.value_expression(left)?, self.value_expression(right)?) {
                    (NumericValueExpr::Int(left), NumericValueExpr::Int(right)) => {
                        Some(BoolExpr::NumberCompare(
                            Box::new(NumberExpr::Int(left)),
                            operation,
                            Box::new(NumberExpr::Int(right)),
                            *span,
                        ))
                    }
                    (NumericValueExpr::Int(left), NumericValueExpr::Float(right)) => {
                        Some(BoolExpr::NumberCompare(
                            Box::new(NumberExpr::Int(left)),
                            operation,
                            Box::new(NumberExpr::Float(right)),
                            *span,
                        ))
                    }
                    (NumericValueExpr::Float(left), NumericValueExpr::Int(right)) => {
                        Some(BoolExpr::NumberCompare(
                            Box::new(NumberExpr::Float(left)),
                            operation,
                            Box::new(NumberExpr::Int(right)),
                            *span,
                        ))
                    }
                    (NumericValueExpr::Float(left), NumericValueExpr::Float(right)) => {
                        Some(BoolExpr::NumberCompare(
                            Box::new(NumberExpr::Float(left)),
                            operation,
                            Box::new(NumberExpr::Float(right)),
                            *span,
                        ))
                    }
                    (NumericValueExpr::Bool(left), NumericValueExpr::Bool(right)) => Some(
                        BoolExpr::BoolCompare(Box::new(left), operation, Box::new(right)),
                    ),
                    _ => None,
                }
            }
            Expr::Call { callee, args, span } => {
                let Expr::Name(name, _) = callee.as_ref() else {
                    return None;
                };
                (self.value_type(expression)? == NumericType::Bool)
                    .then(|| self.numeric_call(name, args, *span).map(BoolExpr::Call))?
            }
            _ => None,
        }
    }

    fn condition_expression(&self, expression: &Expr) -> Option<BoolExpr> {
        match self.value_expression(expression)? {
            NumericValueExpr::Bool(value) => Some(value),
            NumericValueExpr::Int(value) => Some(BoolExpr::IntTruthy(Box::new(value))),
            NumericValueExpr::Float(value) => Some(BoolExpr::FloatTruthy(Box::new(value))),
        }
    }

    fn numeric_call(&self, name: &str, args: &[Expr], span: Span) -> Option<NumericCall> {
        let builtin = !self.environment.functions.contains_key(name);
        let args = args
            .iter()
            .map(|argument| self.value_expression(argument))
            .collect::<Option<Vec<_>>>()?;
        Some(NumericCall {
            name: name.to_owned(),
            args,
            builtin,
            span,
        })
    }

    fn list_method_type(&self, object: &Expr, name: &str) -> Option<NumericType> {
        let Expr::Name(name_of_list, _) = object else {
            return None;
        };
        let list = self.local(name_of_list)?;
        match (list.ty, name) {
            (NumericType::IntList | NumericType::FloatList, "len") => Some(NumericType::Int),
            (NumericType::IntList, "sum" | "product" | "min" | "max") => Some(NumericType::Int),
            (
                NumericType::IntList | NumericType::FloatList,
                "mean" | "median" | "mode" | "variance" | "std",
            )
            | (NumericType::FloatList, "sum" | "product" | "min" | "max") => {
                Some(NumericType::Float)
            }
            _ => None,
        }
    }

    fn list_method_expression(&self, object: &Expr, name: &str, span: Span) -> Option<NumberExpr> {
        let Expr::Name(name_of_list, _) = object else {
            return None;
        };
        let list = self.local(name_of_list)?;
        let list_ref = match list.ty {
            NumericType::IntList => NumericListRef::Int(list.slot),
            NumericType::FloatList => NumericListRef::Float(list.slot),
            _ => return None,
        };
        if name == "len" {
            return Some(NumberExpr::Int(IntExpr::ListLength(list_ref)));
        }
        if list.ty == NumericType::IntList {
            let operation = match name {
                "sum" => Some(IntListOp::Sum),
                "product" => Some(IntListOp::Product),
                "min" => Some(IntListOp::Min),
                "max" => Some(IntListOp::Max),
                _ => None,
            };
            if let Some(operation) = operation {
                return Some(NumberExpr::Int(IntExpr::ListAggregate(
                    list.slot, operation, span,
                )));
            }
        }
        let operation = match name {
            "sum" => FloatListOp::Sum,
            "product" => FloatListOp::Product,
            "min" => FloatListOp::Min,
            "max" => FloatListOp::Max,
            "mean" => FloatListOp::Mean,
            "median" => FloatListOp::Median,
            "mode" => FloatListOp::Mode,
            "variance" => FloatListOp::Variance,
            "std" => FloatListOp::Std,
            _ => return None,
        };
        Some(NumberExpr::Float(FloatExpr::ListAggregate(
            list_ref, operation, span,
        )))
    }

    fn global_list_aggregate(&self, name: &str, args: &[Expr], span: Span) -> Option<NumberExpr> {
        if args.len() != 1 {
            return None;
        }
        let Expr::Name(list_name, _) = &args[0] else {
            return None;
        };
        let local = self.local(list_name)?;
        if !matches!(local.ty, NumericType::IntList | NumericType::FloatList) {
            return None;
        }
        self.list_method_expression(&args[0], name, span)
    }

    fn local(&self, name: &str) -> Option<Local> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    fn declare(&mut self, name: &str, ty: NumericType) -> Option<Local> {
        self.declare_with_constant(name, ty, false)
    }

    fn declare_with_constant(
        &mut self,
        name: &str,
        ty: NumericType,
        constant: bool,
    ) -> Option<Local> {
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
            NumericType::Bool => {
                let slot = self.bool_locals;
                self.bool_locals += 1;
                slot
            }
            NumericType::IntList => {
                let slot = self.int_list_locals;
                self.int_list_locals += 1;
                slot
            }
            NumericType::FloatList => {
                let slot = self.float_list_locals;
                self.float_list_locals += 1;
                slot
            }
        };
        let local = Local { ty, slot, constant };
        scope.insert(name.to_owned(), local);
        Some(local)
    }
}

fn builtin_scalar_type(
    name: &str,
    args: &[Expr],
    type_of: &impl Fn(&Expr) -> Option<NumericType>,
) -> Option<NumericType> {
    match name {
        "sign" | "gcd" | "lcm" | "factorial" => Some(NumericType::Int),
        "hypot" | "atan2" | "clamp" | "pow" => Some(NumericType::Float),
        "isNan" | "isInfinite" | "isFinite" => Some(NumericType::Bool),
        "min" | "max" if !args.is_empty() => {
            let types = args.iter().map(type_of).collect::<Option<Vec<_>>>()?;
            if types.iter().all(|ty| *ty == NumericType::Int) {
                Some(NumericType::Int)
            } else if types.iter().all(|ty| *ty == NumericType::Float) {
                Some(NumericType::Float)
            } else {
                None
            }
        }
        _ => None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_general_numeric_control_flow_and_lists() {
        let source = r#"
fn work(limit: Int, enabled: Bool): Float {
    const scale = 2
    values: List[Float] = []
    i = 0
    loop i < limit {
        if enabled and i % scale == 0 { values.add(i * 1.0) }
        i += 1
    }
    return values.sum()
}
"#;
        let program = crate::parse(source, "numeric-compile-test.shn").unwrap();
        let environment = collect_environment(&program);
        let Stmt::Function { params, body, .. } = &program.statements[0] else {
            panic!("expected a function")
        };
        assert!(compile_function(params, body, &environment).is_some());
    }

    #[test]
    fn rejects_numeric_specialization_that_would_bypass_const_rules() {
        let source = r#"
fn invalid(): Int {
    const value = 1
    value = 2
    return value
}
"#;
        let program = crate::parse(source, "numeric-const-test.shn").unwrap();
        let environment = collect_environment(&program);
        let Stmt::Function { params, body, .. } = &program.statements[0] else {
            panic!("expected a function")
        };
        assert!(compile_function(params, body, &environment).is_none());
    }

    #[test]
    fn dictionaries_use_the_reference_evaluator_fallback() {
        let source = r#"
fn lookup(): Int {
    values: Dictionary[String, Int] = {"answer": 42}
    return values["answer"]
}
"#;
        let program = crate::parse(source, "dictionary-fallback-test.shn").unwrap();
        let environment = collect_environment(&program);
        let Stmt::Function { params, body, .. } = &program.statements[0] else {
            panic!("expected a function")
        };
        assert!(compile_function(params, body, &environment).is_none());
    }
}
