use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::HashMap,
    fmt,
    io::{self, Write},
    rc::Rc,
};

use crate::{ast::*, diagnostics::Diagnostic, lexer::Lexer, parser::Parser, token::Span};

#[derive(Clone)]
pub enum Value {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Rc<RefCell<Vec<Value>>>, bool),
    Range(i64, i64),
    Function(FunctionValue),
    Class(Rc<ClassValue>),
    Instance(Rc<RefCell<InstanceValue>>, bool),
}

#[derive(Clone)]
pub struct FunctionValue {
    params: Vec<Param>,
    return_type: Option<TypeRef>,
    body: Block,
}

#[derive(Clone)]
pub struct ClassValue {
    name: String,
    fields: Vec<FieldDefinition>,
    methods: HashMap<String, MethodDefinition>,
}

#[derive(Clone)]
struct FieldDefinition {
    name: String,
    value: Expr,
    private: bool,
}

#[derive(Clone)]
struct MethodDefinition {
    function: FunctionValue,
    private: bool,
}

pub struct InstanceValue {
    class: Rc<ClassValue>,
    fields: HashMap<String, Value>,
}

impl Value {
    fn type_name(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Bool(_) => "Bool",
            Self::Int(_) => "Int",
            Self::Float(_) => "Float",
            Self::String(_) => "String",
            Self::List(..) => "List",
            Self::Range(..) => "Range",
            Self::Function(_) => "Function",
            Self::Class(_) => "Class",
            Self::Instance(..) => "Object",
        }
    }
    fn truthy(&self) -> bool {
        match self {
            Self::None => false,
            Self::Bool(v) => *v,
            Self::Int(v) => *v != 0,
            Self::Float(v) => *v != 0.0,
            Self::String(v) => !v.is_empty(),
            Self::List(v, _) => !v.borrow().is_empty(),
            Self::Range(a, b) => a != b,
            Self::Function(_) => true,
            Self::Class(_) | Self::Instance(..) => true,
        }
    }
    fn frozen(self) -> Self {
        match self {
            Self::List(v, _) => {
                let values = v.borrow().iter().cloned().map(Value::frozen).collect();
                Self::List(Rc::new(RefCell::new(values)), true)
            }
            Self::Instance(value, _) => Self::Instance(value, true),
            v => v,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => write!(f, "none"),
            Self::Bool(v) => write!(f, "{v}"),
            Self::Int(v) => write!(f, "{v}"),
            Self::Float(v) => {
                if v.is_nan() {
                    write!(f, "NAN")
                } else if v.is_infinite() {
                    if v.is_sign_negative() {
                        write!(f, "-INF")
                    } else {
                        write!(f, "INF")
                    }
                } else {
                    write!(f, "{v}")
                }
            }
            Self::String(v) => write!(f, "{v}"),
            Self::List(v, _) => {
                write!(f, "[")?;
                for (i, x) in v.borrow().iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?
                    }
                    match x {
                        Self::String(s) => write!(f, "\"{s}\"")?,
                        _ => write!(f, "{x}")?,
                    }
                }
                write!(f, "]")
            }
            Self::Range(a, b) => write!(f, "{a}..{b}"),
            Self::Function(_) => write!(f, "<function>"),
            Self::Class(class) => write!(f, "<class {}>", class.name),
            Self::Instance(instance, _) => write!(f, "<{} object>", instance.borrow().class.name),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, o: &Self) -> bool {
        match (self, o) {
            (Self::None, Self::None) => true,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a == b,
            (Self::Int(a), Self::Float(b)) => *a as f64 == *b,
            (Self::Float(a), Self::Int(b)) => *a == *b as f64,
            (Self::String(a), Self::String(b)) => a == b,
            (Self::List(a, _), Self::List(b, _)) => *a.borrow() == *b.borrow(),
            (Self::Class(a), Self::Class(b)) => Rc::ptr_eq(a, b),
            (Self::Instance(a, _), Self::Instance(b, _)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

#[derive(Clone)]
struct Binding {
    value: Value,
    ty: Option<TypeRef>,
    constant: bool,
}
enum Flow {
    Normal,
    Return(Value),
    Break,
    Continue,
}

pub struct Evaluator<'a> {
    source: &'a str,
    file: &'a str,
    output: bool,
    scopes: Vec<HashMap<String, Binding>>,
    call_depth: usize,
    class_stack: Vec<String>,
}

impl<'a> Evaluator<'a> {
    pub fn new(source: &'a str, file: &'a str, output: bool) -> Self {
        let mut global = HashMap::new();
        for (name, value) in [
            ("PI", Value::Float(std::f64::consts::PI)),
            ("TAU", Value::Float(std::f64::consts::TAU)),
            ("E", Value::Float(std::f64::consts::E)),
            ("PHI", Value::Float(1.618_033_988_749_895)),
            ("INF", Value::Float(f64::INFINITY)),
            ("NAN", Value::Float(f64::NAN)),
        ] {
            global.insert(
                name.into(),
                Binding {
                    value,
                    ty: Some(TypeRef::Float),
                    constant: true,
                },
            );
        }
        Self {
            source,
            file,
            output,
            scopes: vec![global, HashMap::new()],
            call_depth: 0,
            class_stack: vec![],
        }
    }

    pub fn run(&mut self, program: &Program) -> Result<(), Diagnostic> {
        self.register_functions(program)?;
        for s in &program.statements {
            if !matches!(s, Stmt::Function { .. } | Stmt::Class { .. }) {
                self.exec(s)?;
            }
        }
        if self.get("main").is_some() {
            self.call_named(
                "main",
                vec![],
                Span {
                    start: 0,
                    length: 1,
                    line: 1,
                    column: 1,
                },
            )?;
        }
        Ok(())
    }
    pub fn check(&mut self, program: &Program) -> Result<(), Diagnostic> {
        self.register_functions(program)?;
        for s in &program.statements {
            if !matches!(s, Stmt::Function { .. } | Stmt::Class { .. }) {
                self.check_stmt(s)?;
            }
        }
        let funcs: Vec<FunctionValue> = self
            .scopes
            .iter()
            .flat_map(|scope| scope.values())
            .filter_map(|b| {
                if let Value::Function(f) = &b.value {
                    Some(f.clone())
                } else {
                    None
                }
            })
            .collect();
        for f in funcs {
            self.push();
            for p in &f.params {
                let v = default_value(p.ty.as_ref());
                self.define(&p.name, v, p.ty.clone(), false, p.span)?;
            }
            for s in &f.body.statements {
                self.check_stmt(s)?;
            }
            self.pop();
        }
        Ok(())
    }

    fn register_functions(&mut self, p: &Program) -> Result<(), Diagnostic> {
        for s in &p.statements {
            if let Stmt::Function {
                name,
                params,
                return_type,
                body,
                span,
            } = s
            {
                self.define(
                    name,
                    Value::Function(FunctionValue {
                        params: params.clone(),
                        return_type: return_type.clone(),
                        body: body.clone(),
                    }),
                    None,
                    true,
                    *span,
                )?;
            } else if let Stmt::Class {
                name,
                members,
                span,
            } = s
            {
                let mut fields = vec![];
                let mut methods = HashMap::new();
                let mut names = std::collections::HashSet::new();
                for member in members {
                    let member_name = match member {
                        ClassMember::Field { name, .. } | ClassMember::Method { name, .. } => name,
                    };
                    if !names.insert(member_name.clone()) {
                        return Err(self.error(
                            "Class Error",
                            format!("duplicate member `{member_name}` in class `{name}`"),
                            *span,
                            "A class member name must be unique.",
                            "Rename or remove the duplicate member.",
                        ));
                    }
                    match member {
                        ClassMember::Field {
                            name,
                            value,
                            private,
                            ..
                        } => fields.push(FieldDefinition {
                            name: name.clone(),
                            value: value.clone(),
                            private: *private,
                        }),
                        ClassMember::Method {
                            name,
                            params,
                            return_type,
                            body,
                            private,
                            ..
                        } => {
                            methods.insert(
                                name.clone(),
                                MethodDefinition {
                                    function: FunctionValue {
                                        params: params.clone(),
                                        return_type: return_type.clone(),
                                        body: body.clone(),
                                    },
                                    private: *private,
                                },
                            );
                        }
                    }
                }
                self.define(
                    name,
                    Value::Class(Rc::new(ClassValue {
                        name: name.clone(),
                        fields,
                        methods,
                    })),
                    None,
                    true,
                    *span,
                )?;
            }
        }
        Ok(())
    }
    fn check_stmt(&mut self, s: &Stmt) -> Result<(), Diagnostic> {
        match s {
            Stmt::If {
                condition,
                then_block,
                else_branch,
                ..
            } => {
                self.eval(condition)?;
                self.push();
                for x in &then_block.statements {
                    self.check_stmt(x)?
                }
                self.pop();
                if let Some(e) = else_branch {
                    self.check_stmt(e)?
                }
                Ok(())
            }
            Stmt::Loop { kind, body, .. } => {
                match kind {
                    LoopKind::Forever => {}
                    LoopKind::While(e) => {
                        self.eval(e)?;
                    }
                    LoopKind::For {
                        name,
                        iterable,
                        step,
                    } => {
                        let iterable_value = self.eval(iterable)?;
                        if let Some(s) = step {
                            self.eval(s)?;
                        }
                        self.push();
                        let item = match iterable_value {
                            Value::List(values, _) => {
                                values.borrow().first().cloned().unwrap_or(Value::None)
                            }
                            Value::String(_) => Value::String(String::new()),
                            Value::Range(_, _) => Value::Int(0),
                            _ => Value::None,
                        };
                        self.define(name, item, None, false, s.span())?;
                        for x in &body.statements {
                            self.check_stmt(x)?
                        }
                        self.pop();
                        return Ok(());
                    }
                }
                self.push();
                for x in &body.statements {
                    self.check_stmt(x)?
                }
                self.pop();
                Ok(())
            }
            Stmt::Return(v, _) => {
                if let Some(v) = v {
                    self.eval(v)?;
                }
                Ok(())
            }
            Stmt::Break(_) | Stmt::Continue(_) | Stmt::Class { .. } => Ok(()),
            _ => self.exec(s).map(|_| ()),
        }
    }

    fn exec(&mut self, s: &Stmt) -> Result<Flow, Diagnostic> {
        match s {
            Stmt::Assign {
                target,
                ty,
                value,
                constant,
                op,
                span,
            } => {
                let mut v = self.eval(value)?;
                if *constant {
                    v = v.frozen();
                }
                self.assign_target(target, ty.clone(), v, *constant, *op, *span)?;
                Ok(Flow::Normal)
            }
            Stmt::Function { .. } | Stmt::Class { .. } => Ok(Flow::Normal),
            Stmt::Expr(e) => {
                self.eval(e)?;
                Ok(Flow::Normal)
            }
            Stmt::Return(v, _) => Ok(Flow::Return(if let Some(e) = v {
                self.eval(e)?
            } else {
                Value::None
            })),
            Stmt::Break(_) => Ok(Flow::Break),
            Stmt::Continue(_) => Ok(Flow::Continue),
            Stmt::If {
                condition,
                then_block,
                else_branch,
                ..
            } => {
                if self.eval(condition)?.truthy() {
                    self.exec_block(then_block)
                } else if let Some(other) = else_branch {
                    self.exec(other)
                } else {
                    Ok(Flow::Normal)
                }
            }
            Stmt::Loop { kind, body, span } => self.exec_loop(kind, body, *span),
        }
    }
    fn exec_block(&mut self, b: &Block) -> Result<Flow, Diagnostic> {
        self.push();
        for s in &b.statements {
            let f = self.exec(s)?;
            if !matches!(f, Flow::Normal) {
                self.pop();
                return Ok(f);
            }
        }
        self.pop();
        Ok(Flow::Normal)
    }
    fn exec_loop(&mut self, k: &LoopKind, b: &Block, span: Span) -> Result<Flow, Diagnostic> {
        match k {
            LoopKind::Forever => loop {
                match self.exec_block(b)? {
                    Flow::Normal | Flow::Continue => {}
                    Flow::Break => break,
                    Flow::Return(v) => return Ok(Flow::Return(v)),
                }
            },
            LoopKind::While(c) => {
                let mut guard = 0usize;
                while self.eval(c)?.truthy() {
                    match self.exec_block(b)? {
                        Flow::Normal | Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                    }
                    guard += 1;
                    if guard > 10_000_000 {
                        return Err(self.error(
                            "Runtime Error",
                            "loop iteration limit exceeded",
                            span,
                            "The loop ran more than ten million times.",
                            "Check that the loop can finish.",
                        ));
                    }
                }
            }
            LoopKind::For {
                name,
                iterable,
                step,
            } => {
                let iterable = self.eval(iterable)?;
                let values = match iterable {
                    Value::Range(a, z) => {
                        let step = if let Some(e) = step {
                            as_int(self.eval(e)?, e.span(), self)?
                        } else if a <= z {
                            1
                        } else {
                            -1
                        };
                        if step == 0 {
                            return Err(self.error(
                                "Value Error",
                                "range step cannot be zero",
                                span,
                                "A zero step never advances.",
                                "Use a positive or negative integer step.",
                            ));
                        }
                        let mut values = vec![];
                        let mut i = a;
                        if step > 0 {
                            while i < z {
                                values.push(Value::Int(i));
                                i = i.checked_add(step).ok_or_else(|| {
                                    self.error(
                                        "Value Error",
                                        "range overflow",
                                        span,
                                        "The range exceeded Int limits.",
                                        "Use a smaller range.",
                                    )
                                })?
                            }
                        } else {
                            while i > z {
                                values.push(Value::Int(i));
                                i = i.checked_add(step).ok_or_else(|| {
                                    self.error(
                                        "Value Error",
                                        "range overflow",
                                        span,
                                        "The range exceeded Int limits.",
                                        "Use a smaller range.",
                                    )
                                })?
                            }
                        }
                        values
                    }
                    Value::List(v, _) => v.borrow().clone(),
                    Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
                    v => return Err(self.type_error(span, "an iterable", &v)),
                };
                for value in values {
                    self.push();
                    self.define(name, value, None, false, span)?;
                    let flow = self.exec_block(b)?;
                    self.pop();
                    match flow {
                        Flow::Normal | Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                    }
                }
            }
        }
        Ok(Flow::Normal)
    }

    fn assign_target(
        &mut self,
        t: &AssignTarget,
        ty: Option<TypeRef>,
        value: Value,
        constant: bool,
        op: AssignOp,
        span: Span,
    ) -> Result<(), Diagnostic> {
        match t {
            AssignTarget::Name(name, name_span) => {
                if matches!(op, AssignOp::Set) {
                    let declaration = ty.is_some() || constant;
                    let local_exists = self
                        .scopes
                        .last()
                        .is_some_and(|scope| scope.contains_key(name));
                    if declaration {
                        if local_exists {
                            return Err(self.error("Name Error",format!("variable `{name}` is already defined"),*name_span,"A variable cannot be declared twice in the same visible scope.","Choose another name or reassign without `const` or a type annotation."));
                        }
                        self.define(name, value, ty, constant, *name_span)
                    } else if self.get(name).is_some() {
                        self.reassign(name, value, *name_span)
                    } else {
                        self.define(name, value, ty, constant, *name_span)
                    }
                } else {
                    let old = self
                        .get(name)
                        .ok_or_else(|| self.unknown(name, *name_span))?
                        .value;
                    let v = self.binary(
                        old,
                        match op {
                            AssignOp::Add => BinaryOp::Add,
                            AssignOp::Subtract => BinaryOp::Subtract,
                            AssignOp::Multiply => BinaryOp::Multiply,
                            AssignOp::Divide => BinaryOp::Divide,
                            AssignOp::Set => unreachable!(),
                        },
                        value,
                        span,
                    )?;
                    self.reassign(name, v, *name_span)
                }
            }
            AssignTarget::Destructure(names, target_span) => {
                let Value::List(items, _) = value else {
                    return Err(self.type_error(*target_span, "a List for destructuring", &value));
                };
                let values = items.borrow();
                if values.len() != names.len() {
                    return Err(self.error(
                        "Value Error",
                        "destructuring length does not match",
                        *target_span,
                        format!(
                            "There are {} names but {} values.",
                            names.len(),
                            values.len()
                        ),
                        "Use the same number of names and values.",
                    ));
                }
                for (name, v) in names.iter().zip(values.iter()) {
                    if self.get(name).is_some() {
                        self.reassign(name, v.clone(), *target_span)?
                    } else {
                        self.define(name, v.clone(), None, constant, *target_span)?
                    }
                }
                Ok(())
            }
            AssignTarget::Index(object, index, target_span) => {
                let expected_item = if let Expr::Name(name, _) = object.as_ref() {
                    self.get(name).and_then(|binding| match binding.ty {
                        Some(TypeRef::List(Some(item))) => Some(*item),
                        _ => None,
                    })
                } else {
                    None
                };
                if let Some(expected) = &expected_item {
                    self.ensure_type(&value, expected, *target_span)?;
                }
                let obj = self.eval(object)?;
                let idx = as_int(self.eval(index)?, index.span(), self)?;
                let Value::List(items, frozen) = obj else {
                    return Err(self.type_error(*target_span, "a List", &obj));
                };
                if frozen {
                    return Err(self.const_error(*target_span));
                }
                let mut values = items.borrow_mut();
                let i = normalize_index(idx, values.len())
                    .ok_or_else(|| self.index_error(*target_span, idx, values.len()))?;
                values[i] = value;
                Ok(())
            }
            AssignTarget::Member(object, name, target_span) => {
                let object = self.eval(object)?;
                let Value::Instance(instance, frozen) = object else {
                    return Err(self.type_error(*target_span, "an Object", &object));
                };
                if frozen {
                    return Err(self.const_error(*target_span));
                }
                let class = instance.borrow().class.clone();
                if class
                    .fields
                    .iter()
                    .any(|field| field.name == *name && field.private)
                    && !self.can_access(&class.name)
                {
                    return Err(self.private_error(&class.name, name, *target_span));
                }
                let old = instance.borrow().fields.get(name).cloned();
                let value = if matches!(op, AssignOp::Set) {
                    value
                } else {
                    let old = old.ok_or_else(|| {
                        self.error(
                            "Name Error",
                            format!("object has no field `{name}`"),
                            *target_span,
                            "Compound assignment requires an existing field.",
                            "Assign the field with `=` first.",
                        )
                    })?;
                    self.binary(
                        old,
                        match op {
                            AssignOp::Add => BinaryOp::Add,
                            AssignOp::Subtract => BinaryOp::Subtract,
                            AssignOp::Multiply => BinaryOp::Multiply,
                            AssignOp::Divide => BinaryOp::Divide,
                            AssignOp::Set => unreachable!(),
                        },
                        value,
                        span,
                    )?
                };
                instance.borrow_mut().fields.insert(name.clone(), value);
                Ok(())
            }
        }
    }

    fn eval(&mut self, e: &Expr) -> Result<Value, Diagnostic> {
        match e {
            Expr::None(_) => Ok(Value::None),
            Expr::Bool(v, _) => Ok(Value::Bool(*v)),
            Expr::Int(v, _) => Ok(Value::Int(*v)),
            Expr::Float(v, _) => Ok(Value::Float(*v)),
            Expr::String(v, s) => self.interpolate(v, *s),
            Expr::List(items, _) => Ok(Value::List(
                Rc::new(RefCell::new(
                    items
                        .iter()
                        .map(|x| self.eval(x))
                        .collect::<Result<Vec<_>, _>>()?,
                )),
                false,
            )),
            Expr::Name(n, s) => self
                .get(n)
                .map(|b| b.value)
                .ok_or_else(|| self.unknown(n, *s)),
            Expr::Unary { op, value, span } => {
                let v = self.eval(value)?;
                match op {
                    UnaryOp::Not => Ok(Value::Bool(!v.truthy())),
                    UnaryOp::Positive => {
                        if matches!(v, Value::Int(_) | Value::Float(_)) {
                            Ok(v)
                        } else {
                            Err(self.type_error(*span, "a Number", &v))
                        }
                    }
                    UnaryOp::Negate => match v {
                        Value::Int(x) => x.checked_neg().map(Value::Int).ok_or_else(|| {
                            self.error(
                                "Value Error",
                                "integer overflow",
                                *span,
                                "This result is outside Int limits.",
                                "Use a Float or smaller value.",
                            )
                        }),
                        Value::Float(x) => Ok(Value::Float(-x)),
                        v => Err(self.type_error(*span, "a Number", &v)),
                    },
                }
            }
            Expr::Binary {
                left,
                op,
                right,
                span,
            } => {
                if matches!(op, BinaryOp::And) {
                    let l = self.eval(left)?;
                    return if !l.truthy() {
                        Ok(Value::Bool(false))
                    } else {
                        Ok(Value::Bool(self.eval(right)?.truthy()))
                    };
                }
                if matches!(op, BinaryOp::Or) {
                    let l = self.eval(left)?;
                    return if l.truthy() {
                        Ok(Value::Bool(true))
                    } else {
                        Ok(Value::Bool(self.eval(right)?.truthy()))
                    };
                }
                let l = self.eval(left)?;
                let r = self.eval(right)?;
                self.binary(l, *op, r, *span)
            }
            Expr::Range { start, end, span } => {
                let a = as_int(self.eval(start)?, start.span(), self)?;
                let b = as_int(self.eval(end)?, end.span(), self)?;
                let _ = span;
                Ok(Value::Range(a, b))
            }
            Expr::Index {
                object,
                index,
                span,
            } => {
                let obj = self.eval(object)?;
                let idx = as_int(self.eval(index)?, index.span(), self)?;
                match obj {
                    Value::List(v, _) => {
                        let values = v.borrow();
                        let i = normalize_index(idx, values.len())
                            .ok_or_else(|| self.index_error(*span, idx, values.len()))?;
                        Ok(values[i].clone())
                    }
                    Value::String(v) => {
                        let chars: Vec<_> = v.chars().collect();
                        let i = normalize_index(idx, chars.len())
                            .ok_or_else(|| self.index_error(*span, idx, chars.len()))?;
                        Ok(Value::String(chars[i].to_string()))
                    }
                    v => Err(self.type_error(*span, "a List or String", &v)),
                }
            }
            Expr::Slice {
                object,
                start,
                end,
                span,
            } => {
                let obj = self.eval(object)?;
                let a = if let Some(x) = start {
                    Some(as_int(self.eval(x)?, x.span(), self)?)
                } else {
                    None
                };
                let z = if let Some(x) = end {
                    Some(as_int(self.eval(x)?, x.span(), self)?)
                } else {
                    None
                };
                self.slice(obj, a, z, *span)
            }
            Expr::Call { callee, args, span } => {
                let values = args
                    .iter()
                    .map(|x| self.eval(x))
                    .collect::<Result<Vec<_>, _>>()?;
                if let Expr::Name(name, _) = callee.as_ref() {
                    if self.get(name).is_none() {
                        return self.call_builtin(name, values, *span);
                    }
                    self.call_named(name, values, *span)
                } else {
                    let f = self.eval(callee)?;
                    self.call_value(f, values, *span)
                }
            }
            Expr::MemberCall {
                object,
                name,
                args,
                span,
            } => {
                let expected_item = if let Expr::Name(object_name, _) = object.as_ref() {
                    self.get(object_name).and_then(|binding| match binding.ty {
                        Some(TypeRef::List(Some(item))) => Some(*item),
                        _ => None,
                    })
                } else {
                    None
                };
                let obj = self.eval(object)?;
                let values = args
                    .iter()
                    .map(|x| self.eval(x))
                    .collect::<Result<Vec<_>, _>>()?;
                self.member(obj, name, values, expected_item.as_ref(), *span)
            }
            Expr::Member { object, name, span } => {
                let object = self.eval(object)?;
                let Value::Instance(instance, _) = object else {
                    return Err(self.type_error(*span, "an Object", &object));
                };
                let class = instance.borrow().class.clone();
                if class
                    .fields
                    .iter()
                    .any(|field| field.name == *name && field.private)
                    && !self.can_access(&class.name)
                {
                    return Err(self.private_error(&class.name, name, *span));
                }
                let value = instance.borrow().fields.get(name).cloned();
                value.ok_or_else(|| {
                    self.error(
                        "Name Error",
                        format!("{} has no field `{name}`", class.name),
                        *span,
                        "The requested object field does not exist.",
                        "Check the field name.",
                    )
                })
            }
        }
    }

    fn binary(&self, l: Value, op: BinaryOp, r: Value, span: Span) -> Result<Value, Diagnostic> {
        use BinaryOp::*;
        match op {
            Equal => Ok(Value::Bool(l == r)),
            NotEqual => Ok(Value::Bool(l != r)),
            And | Or => unreachable!(),
            In => match r {
                Value::List(v, _) => Ok(Value::Bool(v.borrow().contains(&l))),
                Value::String(s) => {
                    if let Value::String(x) = l {
                        Ok(Value::Bool(s.contains(&x)))
                    } else {
                        Err(self.type_error(span, "a String membership value", &l))
                    }
                }
                v => Err(self.type_error(span, "a List or String after `in`", &v)),
            },
            Add => match (l, r) {
                (Value::String(a), Value::String(b)) => Ok(Value::String(a + &b)),
                (Value::List(a, _), Value::List(b, _)) => {
                    let mut v = a.borrow().clone();
                    v.extend(b.borrow().clone());
                    Ok(Value::List(Rc::new(RefCell::new(v)), false))
                }
                (a, b) => numeric2(a, b, span, self, |a, b| a + b, |a, b| a.checked_add(b)),
            },
            Subtract => numeric2(l, r, span, self, |a, b| a - b, |a, b| a.checked_sub(b)),
            Multiply => match (l, r) {
                (Value::String(s), Value::Int(n)) | (Value::Int(n), Value::String(s)) => {
                    repeat_string(s, n, span, self)
                }
                (Value::List(v, _), Value::Int(n)) | (Value::Int(n), Value::List(v, _)) => {
                    repeat_list(v, n, span, self)
                }
                (a, b) => numeric2(a, b, span, self, |a, b| a * b, |a, b| a.checked_mul(b)),
            },
            Divide => {
                let (a, b) = numbers(l, r, span, self)?;
                if b == 0.0 {
                    return Err(self.zero(span));
                }
                Ok(Value::Float(a / b))
            }
            IntegerDivide => match (l, r) {
                (Value::Int(a), Value::Int(b)) => {
                    if b == 0 {
                        Err(self.zero(span))
                    } else {
                        a.checked_div(b).map(Value::Int).ok_or_else(|| {
                            self.error(
                                "Value Error",
                                "integer overflow",
                                span,
                                "The division result is outside Int limits.",
                                "Use `/` for floating-point division.",
                            )
                        })
                    }
                }
                (a, b) => {
                    let (x, y) = numbers(a, b, span, self)?;
                    if y == 0.0 {
                        Err(self.zero(span))
                    } else {
                        Ok(Value::Float((x / y).floor()))
                    }
                }
            },
            Remainder => match (l, r) {
                (Value::Int(a), Value::Int(b)) => {
                    if b == 0 {
                        Err(self.zero(span))
                    } else {
                        Ok(Value::Int(a % b))
                    }
                }
                (a, b) => {
                    let (x, y) = numbers(a, b, span, self)?;
                    if y == 0.0 {
                        Err(self.zero(span))
                    } else {
                        Ok(Value::Float(x % y))
                    }
                }
            },
            Power => {
                let (a, b) = numbers(l, r, span, self)?;
                let result = a.powf(b);
                if result.is_nan() {
                    Err(self.error(
                        "Math Error",
                        "power produced an invalid result",
                        span,
                        "The base and exponent are outside the real-number domain.",
                        "Use values that produce a real result.",
                    ))
                } else {
                    Ok(Value::Float(result))
                }
            }
            Less | LessEqual | Greater | GreaterEqual => {
                let ord = compare(&l, &r).ok_or_else(|| {
                    self.error(
                        "Type Error",
                        "values cannot be ordered",
                        span,
                        format!(
                            "{} and {} do not have a common ordering.",
                            l.type_name(),
                            r.type_name()
                        ),
                        "Compare two numbers or two strings.",
                    )
                })?;
                Ok(Value::Bool(match op {
                    Less => ord == Ordering::Less,
                    LessEqual => ord != Ordering::Greater,
                    Greater => ord == Ordering::Greater,
                    GreaterEqual => ord != Ordering::Less,
                    _ => false,
                }))
            }
        }
    }

    fn call_named(&mut self, n: &str, args: Vec<Value>, span: Span) -> Result<Value, Diagnostic> {
        let value = self.get(n).ok_or_else(|| self.unknown(n, span))?.value;
        self.call_value(value, args, span)
    }
    fn call_value(&mut self, v: Value, args: Vec<Value>, span: Span) -> Result<Value, Diagnostic> {
        match v {
            Value::Function(function) => self.call_function(function, args, None, None, span),
            Value::Class(class) => self.instantiate(class, args, span),
            value => Err(self.type_error(span, "a Function or Class", &value)),
        }
    }

    fn call_function(
        &mut self,
        f: FunctionValue,
        args: Vec<Value>,
        receiver: Option<Value>,
        owner: Option<String>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if args.len() != f.params.len() {
            return Err(self.error(
                "Argument Error",
                format!(
                    "expected {} arguments, received {}",
                    f.params.len(),
                    args.len()
                ),
                span,
                "The call does not match the function parameters.",
                "Pass the required number of arguments.",
            ));
        }
        self.call_depth += 1;
        if self.call_depth > 1000 {
            return Err(self.error(
                "Runtime Error",
                "maximum call depth exceeded",
                span,
                "The function calls are too deeply recursive.",
                "Add a stopping condition.",
            ));
        }
        self.push();
        if let Some(receiver) = receiver {
            self.define("self", receiver, None, true, span)?;
        }
        if let Some(owner) = &owner {
            self.class_stack.push(owner.clone());
        }
        for (p, v) in f.params.iter().zip(args) {
            self.define(&p.name, v, p.ty.clone(), false, p.span)?
        }
        let mut result = Value::None;
        for s in &f.body.statements {
            match self.exec(s)? {
                Flow::Normal => {}
                Flow::Return(v) => {
                    result = v;
                    break;
                }
                Flow::Break | Flow::Continue => {
                    return Err(self.error(
                        "Control Error",
                        "loop control used outside a loop",
                        s.span(),
                        "break and continue only make sense inside loop.",
                        "Move it into a loop.",
                    ))
                }
            }
        }
        self.pop();
        if owner.is_some() {
            self.class_stack.pop();
        }
        self.call_depth -= 1;
        if let Some(ty) = &f.return_type {
            self.ensure_type(&result, ty, span)?
        }
        Ok(result)
    }

    fn instantiate(
        &mut self,
        class: Rc<ClassValue>,
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let mut fields = HashMap::new();
        for field in &class.fields {
            fields.insert(field.name.clone(), self.eval(&field.value)?);
        }
        let instance = Value::Instance(
            Rc::new(RefCell::new(InstanceValue {
                class: class.clone(),
                fields,
            })),
            false,
        );
        if let Some(initializer) = class.methods.get("init") {
            self.call_function(
                initializer.function.clone(),
                args,
                Some(instance.clone()),
                Some(class.name.clone()),
                span,
            )?;
        } else if !args.is_empty() {
            return Err(self.arg_error(span, &class.name, 0, args.len()));
        }
        Ok(instance)
    }

    fn member(
        &mut self,
        obj: Value,
        name: &str,
        args: Vec<Value>,
        expected_item: Option<&TypeRef>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        if let Value::Instance(instance, frozen) = &obj {
            let class = instance.borrow().class.clone();
            let method = class.methods.get(name).cloned().ok_or_else(|| {
                self.error(
                    "Name Error",
                    format!("{} has no method `{name}`", class.name),
                    span,
                    "The requested object method does not exist.",
                    "Check the method name.",
                )
            })?;
            if method.private && !self.can_access(&class.name) {
                return Err(self.private_error(&class.name, name, span));
            }
            return self.call_function(
                method.function,
                args,
                Some(Value::Instance(instance.clone(), *frozen)),
                Some(class.name.clone()),
                span,
            );
        }
        let Value::List(items, frozen) = obj else {
            return Err(self.type_error(span, "a List or Object before the method", &obj));
        };
        let mut v = items.borrow_mut();
        match name{
        "add"=>{
            if frozen{return Err(self.const_error(span))}
            if args.is_empty(){return Err(self.arg_error(span,"add",1,args.len()))}
            if let Some(expected) = expected_item {
                for value in &args {
                    self.ensure_type(value, expected, span)?;
                }
            }
            v.extend(args);Ok(Value::None)
        },
        "del"=>{if frozen{return Err(self.const_error(span))}one_arg(&args,span,"del",self)?;let i=as_int(args[0].clone(),span,self)?;let i=normalize_index(i,v.len()).ok_or_else(||self.index_error(span,i,v.len()))?;Ok(v.remove(i))},
        "remove"=>{if frozen{return Err(self.const_error(span))}one_arg(&args,span,"remove",self)?;if let Some(i)=v.iter().position(|x|x==&args[0]){v.remove(i);Ok(Value::Bool(true))}else{Ok(Value::Bool(false))}},
        "have"=>{one_arg(&args,span,"have",self)?;Ok(Value::Bool(v.contains(&args[0])))},
        "index"=>{one_arg(&args,span,"index",self)?;Ok(v.iter().position(|x|x==&args[0]).map(|i|Value::Int(i as i64)).unwrap_or(Value::Bool(false)))},
        "len"=>{zero_args(&args,span,"len",self)?;Ok(Value::Int(v.len() as i64))},
        "clear"=>{zero_args(&args,span,"clear",self)?;if frozen{return Err(self.const_error(span))}v.clear();Ok(Value::None)},
        "copy"=>{zero_args(&args,span,"copy",self)?;Ok(Value::List(Rc::new(RefCell::new(v.clone())),false))},
        "unique"=>{zero_args(&args,span,"unique",self)?;let mut out=vec![];for x in v.iter(){if !out.contains(x){out.push(x.clone())}}Ok(Value::List(Rc::new(RefCell::new(out)),false))},
        "reverse"=>{zero_args(&args,span,"reverse",self)?;if frozen{return Err(self.const_error(span))}v.reverse();Ok(Value::None)},
        "sort"=>{zero_args(&args,span,"sort",self)?;if frozen{return Err(self.const_error(span))}v.sort_by(|a,b|compare(a,b).unwrap_or(Ordering::Equal));Ok(Value::None)},
        "sum"|"product"|"min"|"max"|"mean"|"median"|"mode"|"variance"|"std"=>{zero_args(&args,span,name,self)?;aggregate(name,&v,span,self)},
        _=>Err(self.error("Name Error",format!("List has no method `{name}`"),span,"The requested list method does not exist.","Use add, del, remove, have, index, len, clear, copy, unique, reverse, sort, sum, min, max, or mean.")),
    }
    }

    fn call_builtin(&mut self, n: &str, a: Vec<Value>, span: Span) -> Result<Value, Diagnostic> {
        match n {
            "assert" => {
                if a.is_empty() || a.len() > 2 {
                    return Err(self.error(
                        "Argument Error",
                        "assert expects one or two arguments",
                        span,
                        "Use assert(condition) or assert(condition, message).",
                        "Pass a condition and optional message.",
                    ));
                }
                if !a[0].truthy() {
                    let message = a
                        .get(1)
                        .map(ToString::to_string)
                        .unwrap_or_else(|| "assertion failed".into());
                    return Err(self.error(
                        "Assertion Error",
                        message,
                        span,
                        "A test condition evaluated to false.",
                        "Fix the program or expected value.",
                    ));
                }
                Ok(Value::None)
            }
            "print" => {
                if a.is_empty() {
                    if self.output {
                        println!()
                    }
                    return Ok(Value::None);
                }
                if self.output {
                    for (i, v) in a.iter().enumerate() {
                        if i > 0 {
                            print!(" ")
                        }
                        print!("{v}")
                    }
                    println!()
                }
                Ok(Value::None)
            }
            "input" => {
                if a.len() > 1 {
                    return Err(self.arg_error(span, n, 1, a.len()));
                }
                if self.output {
                    if let Some(v) = a.first() {
                        print!("{v}");
                        io::stdout().flush().ok();
                    }
                    let mut s = String::new();
                    io::stdin().read_line(&mut s).map_err(|e| {
                        self.error(
                            "Input Error",
                            "could not read input",
                            span,
                            e.to_string(),
                            "Try the command again.",
                        )
                    })?;
                    Ok(Value::String(s.trim_end_matches(['\n', '\r']).to_string()))
                } else {
                    Ok(Value::String(String::new()))
                }
            }
            "length" => {
                one_arg(&a, span, n, self)?;
                match &a[0] {
                    Value::List(v, _) => Ok(Value::Int(v.borrow().len() as i64)),
                    Value::String(v) => Ok(Value::Int(v.chars().count() as i64)),
                    v => Err(self.type_error(span, "a List or String", v)),
                }
            }
            "type" => {
                one_arg(&a, span, n, self)?;
                Ok(Value::String(a[0].type_name().into()))
            }
            "string" => {
                one_arg(&a, span, n, self)?;
                Ok(Value::String(a[0].to_string()))
            }
            "bool" => {
                one_arg(&a, span, n, self)?;
                Ok(Value::Bool(a[0].truthy()))
            }
            "number" => {
                one_arg(&a, span, n, self)?;
                match &a[0] {
                    Value::Int(v) => Ok(Value::Int(*v)),
                    Value::Float(v) => Ok(Value::Float(*v)),
                    Value::Bool(v) => Ok(Value::Int(*v as i64)),
                    Value::String(s) => {
                        if let Ok(v) = s.parse::<i64>() {
                            Ok(Value::Int(v))
                        } else if let Ok(v) = s.parse::<f64>() {
                            Ok(Value::Float(v))
                        } else {
                            Err(self.error(
                                "Conversion Error",
                                format!("cannot convert `{s}` to Number"),
                                span,
                                "The string does not contain a valid number.",
                                "Use digits with an optional decimal point or exponent.",
                            ))
                        }
                    }
                    v => Err(self.type_error(span, "a String, Bool, or Number", v)),
                }
            }
            "readFile" => {
                one_arg(&a, span, n, self)?;
                let p = as_string(&a[0], span, self)?;
                if !self.output {
                    return Ok(Value::String(String::new()));
                }
                std::fs::read_to_string(p).map(Value::String).map_err(|e| {
                    self.error(
                        "File Error",
                        format!("could not read `{p}`"),
                        span,
                        e.to_string(),
                        "Check that the path exists and is readable.",
                    )
                })
            }
            "writeFile" => {
                if a.len() != 2 {
                    return Err(self.arg_error(span, n, 2, a.len()));
                }
                let p = as_string(&a[0], span, self)?;
                let text = as_string(&a[1], span, self)?;
                if self.output {
                    std::fs::write(p, text).map_err(|e| {
                        self.error(
                            "File Error",
                            format!("could not write `{p}`"),
                            span,
                            e.to_string(),
                            "Check the path and directory permissions.",
                        )
                    })?
                }
                Ok(Value::None)
            }
            "abs" => unary_math(n, a, span, self, |x| x.abs()),
            "floor" => unary_math(n, a, span, self, |x| x.floor()),
            "ceil" => unary_math(n, a, span, self, |x| x.ceil()),
            "sqrt" => unary_math(n, a, span, self, |x| x.sqrt()),
            "sin" => unary_math(n, a, span, self, |x| x.sin()),
            "cos" => unary_math(n, a, span, self, |x| x.cos()),
            "tan" => unary_math(n, a, span, self, |x| x.tan()),
            "asin" => unary_math(n, a, span, self, |x| x.asin()),
            "acos" => unary_math(n, a, span, self, |x| x.acos()),
            "atan" => unary_math(n, a, span, self, |x| x.atan()),
            "log" => unary_math(n, a, span, self, |x| x.ln()),
            "log10" => unary_math(n, a, span, self, |x| x.log10()),
            "log2" => unary_math(n, a, span, self, |x| x.log2()),
            "exp" => unary_math(n, a, span, self, |x| x.exp()),
            "exp2" => unary_math(n, a, span, self, |x| x.exp2()),
            "cbrt" => unary_math(n, a, span, self, |x| x.cbrt()),
            "trunc" => unary_math(n, a, span, self, |x| x.trunc()),
            "fract" => unary_math(n, a, span, self, |x| x.fract()),
            "sinh" => unary_math(n, a, span, self, |x| x.sinh()),
            "cosh" => unary_math(n, a, span, self, |x| x.cosh()),
            "tanh" => unary_math(n, a, span, self, |x| x.tanh()),
            "asinh" => unary_math(n, a, span, self, |x| x.asinh()),
            "acosh" => unary_math(n, a, span, self, |x| x.acosh()),
            "atanh" => unary_math(n, a, span, self, |x| x.atanh()),
            "degrees" => unary_math(n, a, span, self, |x| x.to_degrees()),
            "radians" => unary_math(n, a, span, self, |x| x.to_radians()),
            "hypot" => binary_math(n, a, span, self, |x, y| x.hypot(y)),
            "atan2" => binary_math(n, a, span, self, |y, x| y.atan2(x)),
            "sign" => {
                one_arg(&a, span, n, self)?;
                let value = as_number(&a[0], span, self)?;
                if value.is_nan() {
                    return Err(self.error(
                        "Math Error",
                        "sign is undefined for NAN",
                        span,
                        "NAN has no sign.",
                        "Use a finite number.",
                    ));
                }
                Ok(Value::Int(if value > 0.0 {
                    1
                } else if value < 0.0 {
                    -1
                } else {
                    0
                }))
            }
            "isNan" | "isInfinite" | "isFinite" => {
                one_arg(&a, span, n, self)?;
                let value = as_number(&a[0], span, self)?;
                Ok(Value::Bool(match n {
                    "isNan" => value.is_nan(),
                    "isInfinite" => value.is_infinite(),
                    "isFinite" => value.is_finite(),
                    _ => unreachable!(),
                }))
            }
            "clamp" => {
                if a.len() != 3 {
                    return Err(self.arg_error(span, n, 3, a.len()));
                }
                let value = as_number(&a[0], span, self)?;
                let minimum = as_number(&a[1], span, self)?;
                let maximum = as_number(&a[2], span, self)?;
                if minimum > maximum {
                    return Err(self.error(
                        "Value Error",
                        "clamp minimum exceeds maximum",
                        span,
                        "The lower bound must not exceed the upper bound.",
                        "Swap or correct the bounds.",
                    ));
                }
                Ok(Value::Float(value.clamp(minimum, maximum)))
            }
            "gcd" | "lcm" => {
                if a.len() != 2 {
                    return Err(self.arg_error(span, n, 2, a.len()));
                }
                let left = as_int(a[0].clone(), span, self)?;
                let right = as_int(a[1].clone(), span, self)?;
                integer_pair_math(n, left, right, span, self)
            }
            "factorial" => {
                one_arg(&a, span, n, self)?;
                let value = as_int(a[0].clone(), span, self)?;
                if !(0..=20).contains(&value) {
                    return Err(self.error(
                        "Value Error",
                        "factorial supports Int values from 0 through 20",
                        span,
                        "Larger factorials exceed the current Int range.",
                        "Use an Int between 0 and 20.",
                    ));
                }
                Ok(Value::Int((1..=value).product()))
            }
            "pow" => {
                if a.len() != 2 {
                    return Err(self.arg_error(span, n, 2, a.len()));
                }
                let (x, y) = numbers(a[0].clone(), a[1].clone(), span, self)?;
                let result = x.powf(y);
                if result.is_nan() {
                    Err(self.error(
                        "Math Error",
                        "pow produced an invalid result",
                        span,
                        "The base and exponent are outside the real-number domain.",
                        "Use values that produce a real result.",
                    ))
                } else {
                    Ok(Value::Float(result))
                }
            }
            "round" => {
                if !(a.len() == 1 || a.len() == 2) {
                    return Err(self.error(
                        "Argument Error",
                        "round expects one or two arguments",
                        span,
                        "Use round(value) or round(value, digits).",
                        "Pass a number and optional digit count.",
                    ));
                }
                let x = as_number(&a[0], span, self)?;
                let digits = if a.len() == 2 {
                    as_int(a[1].clone(), span, self)?
                } else {
                    0
                };
                let factor = 10f64.powi(digits.try_into().map_err(|_| {
                    self.error(
                        "Value Error",
                        "round digits are too large",
                        span,
                        "The digit count does not fit a 32-bit integer.",
                        "Use a smaller digit count.",
                    )
                })?);
                let out = (x * factor).round() / factor;
                if digits == 0 {
                    Ok(Value::Int(out as i64))
                } else {
                    Ok(Value::Float(out))
                }
            }
            "min" | "max" => {
                if a.is_empty() {
                    return Err(self.arg_error(span, n, 1, 0));
                }
                let mut best = a[0].clone();
                for x in &a[1..] {
                    let ord = compare(x, &best)
                        .ok_or_else(|| self.type_error(span, "comparable values", x))?;
                    if (n == "min" && ord == Ordering::Less)
                        || (n == "max" && ord == Ordering::Greater)
                    {
                        best = x.clone()
                    }
                }
                Ok(best)
            }
            "sum" | "product" | "mean" | "median" | "mode" | "variance" | "std" => {
                one_arg(&a, span, n, self)?;
                let Value::List(v, _) = &a[0] else {
                    return Err(self.type_error(span, "a List", &a[0]));
                };
                aggregate(n, &v.borrow(), span, self)
            }
            _ => Err(self.unknown(n, span)),
        }
    }

    fn interpolate(&mut self, text: &str, span: Span) -> Result<Value, Diagnostic> {
        let mut out = String::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '{' {
                if i + 1 < chars.len() && chars[i + 1] == '{' {
                    out.push('{');
                    i += 2;
                    continue;
                }
                let start = i + 1;
                let mut depth = 1;
                i += 1;
                while i < chars.len() && depth > 0 {
                    if chars[i] == '{' {
                        depth += 1
                    } else if chars[i] == '}' {
                        depth -= 1
                    }
                    if depth > 0 {
                        i += 1
                    }
                }
                if depth != 0 {
                    return Err(self.error(
                        "String Error",
                        "unterminated interpolation",
                        span,
                        "An opening `{` has no matching `}`.",
                        "Add the closing brace or use `{{` for a literal brace.",
                    ));
                }
                let expr: String = chars[start..i].iter().collect();
                let tokens = Lexer::new(&expr, self.file).scan().map_err(|_| {
                    self.error(
                        "String Error",
                        "invalid interpolation",
                        span,
                        format!("`{expr}` is not a valid expression."),
                        "Put a valid Shine expression inside the braces.",
                    )
                })?;
                let parsed = Parser::new(tokens, &expr, self.file)
                    .expression_only()
                    .map_err(|_| {
                        self.error(
                            "String Error",
                            "invalid interpolation",
                            span,
                            format!("`{expr}` is not a valid expression."),
                            "Put a valid Shine expression inside the braces.",
                        )
                    })?;
                out.push_str(&self.eval(&parsed)?.to_string());
                i += 1
            } else if chars[i] == '}' && i + 1 < chars.len() && chars[i + 1] == '}' {
                out.push('}');
                i += 2
            } else {
                out.push(chars[i]);
                i += 1
            }
        }
        Ok(Value::String(out))
    }

    fn slice(
        &self,
        obj: Value,
        a: Option<i64>,
        z: Option<i64>,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        match obj {
            Value::List(v, _) => {
                let values = v.borrow();
                let (s, e) = slice_bounds(a, z, values.len(), span, self)?;
                Ok(Value::List(
                    Rc::new(RefCell::new(values[s..e].to_vec())),
                    false,
                ))
            }
            Value::String(v) => {
                let c: Vec<char> = v.chars().collect();
                let (s, e) = slice_bounds(a, z, c.len(), span, self)?;
                Ok(Value::String(c[s..e].iter().collect()))
            }
            v => Err(self.type_error(span, "a List or String", &v)),
        }
    }
    fn define(
        &mut self,
        n: &str,
        v: Value,
        ty: Option<TypeRef>,
        constant: bool,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if let Some(t) = &ty {
            self.ensure_type(&v, t, span)?
        }
        let scope = self.scopes.last_mut().unwrap();
        if scope.contains_key(n) {
            return Err(self.error(
                "Name Error",
                format!("`{n}` is already defined in this scope"),
                span,
                "Names must be unique inside one scope.",
                "Reassign it without a declaration or choose another name.",
            ));
        }
        scope.insert(
            n.into(),
            Binding {
                value: v,
                ty,
                constant,
            },
        );
        Ok(())
    }
    fn reassign(&mut self, n: &str, v: Value, span: Span) -> Result<(), Diagnostic> {
        for i in (0..self.scopes.len()).rev() {
            if let Some(b) = self.scopes[i].get(n).cloned() {
                if b.constant {
                    return Err(self.error(
                        "Const Error",
                        format!("cannot reassign constant `{n}`"),
                        span,
                        "Constants cannot be reassigned or mutated.",
                        "Create a new variable instead.",
                    ));
                }
                if let Some(t) = &b.ty {
                    self.ensure_type(&v, t, span)?
                }
                self.scopes[i].get_mut(n).unwrap().value = v;
                return Ok(());
            }
        }
        Err(self.unknown(n, span))
    }
    fn ensure_type(&self, v: &Value, t: &TypeRef, span: Span) -> Result<(), Diagnostic> {
        let good = match t {
            TypeRef::Int => matches!(v, Value::Int(_)),
            TypeRef::Float => matches!(v, Value::Float(_)),
            TypeRef::Number => matches!(v, Value::Int(_) | Value::Float(_)),
            TypeRef::String => matches!(v, Value::String(_)),
            TypeRef::Bool => matches!(v, Value::Bool(_)),
            TypeRef::None => matches!(v, Value::None),
            TypeRef::List(item) => {
                if let Value::List(values, _) = v {
                    item.as_ref()
                        .map(|t| {
                            values
                                .borrow()
                                .iter()
                                .all(|v| self.ensure_type(v, t, span).is_ok())
                        })
                        .unwrap_or(true)
                } else {
                    false
                }
            }
        };
        if good {
            Ok(())
        } else {
            Err(self.error(
                "Type Error",
                format!("expected {}, received {}", type_ref_name(t), v.type_name()),
                span,
                "A fixed type cannot receive this value.",
                "Use a matching value or remove the type annotation.",
            ))
        }
    }
    fn get(&self, n: &str) -> Option<Binding> {
        self.scopes.iter().rev().find_map(|s| s.get(n).cloned())
    }
    fn push(&mut self) {
        self.scopes.push(HashMap::new())
    }
    fn pop(&mut self) {
        self.scopes.pop();
    }
    fn error(
        &self,
        c: impl Into<String>,
        m: impl Into<String>,
        s: Span,
        e: impl Into<String>,
        g: impl Into<String>,
    ) -> Diagnostic {
        Diagnostic::at(c, m, self.file, self.source, s, e, g)
    }
    fn unknown(&self, n: &str, s: Span) -> Diagnostic {
        self.error(
            "Name Error",
            format!("unknown name `{n}`"),
            s,
            "No variable, function, or built-in with this name is visible here.",
            "Define it before use or check the spelling.",
        )
    }
    fn type_error(&self, s: Span, expected: &str, v: &Value) -> Diagnostic {
        self.error(
            "Type Error",
            format!("expected {expected}, received {}", v.type_name()),
            s,
            "This operation does not support the supplied value.",
            "Use a value of the expected type.",
        )
    }
    fn const_error(&self, s: Span) -> Diagnostic {
        self.error(
            "Const Error",
            "cannot mutate a constant value",
            s,
            "A const protects both its binding and contained values.",
            "Call copy() first or use a non-const variable.",
        )
    }
    fn can_access(&self, class: &str) -> bool {
        self.class_stack
            .last()
            .is_some_and(|current| current == class)
    }
    fn private_error(&self, class: &str, member: &str, span: Span) -> Diagnostic {
        self.error(
            "Access Error",
            format!("`{member}` is private in class `{class}`"),
            span,
            "Private members are accessible only from methods of their class.",
            "Use a public method provided by the class.",
        )
    }
    fn index_error(&self, s: Span, i: i64, len: usize) -> Diagnostic {
        self.error(
            "Index Error",
            format!("index {i} is outside a list of length {len}"),
            s,
            "Valid indexes must refer to an existing element.",
            "Use an index between 0 and length - 1.",
        )
    }
    fn zero(&self, s: Span) -> Diagnostic {
        self.error(
            "Math Error",
            "division by zero",
            s,
            "A number cannot be divided by zero.",
            "Ensure the divisor is non-zero.",
        )
    }
    fn arg_error(&self, s: Span, n: &str, want: usize, got: usize) -> Diagnostic {
        self.error(
            "Argument Error",
            format!("{n} expects {want} argument(s), received {got}"),
            s,
            "The call has the wrong number of arguments.",
            "Adjust the arguments to match the function.",
        )
    }
}

fn default_value(t: Option<&TypeRef>) -> Value {
    match t {
        Some(TypeRef::Int) => Value::Int(0),
        Some(TypeRef::Float | TypeRef::Number) => Value::Float(0.0),
        Some(TypeRef::String) => Value::String(String::new()),
        Some(TypeRef::Bool) => Value::Bool(false),
        Some(TypeRef::List(_)) => Value::List(Rc::new(RefCell::new(vec![])), false),
        _ => Value::None,
    }
}
fn type_ref_name(t: &TypeRef) -> String {
    match t {
        TypeRef::Int => "Int".into(),
        TypeRef::Float => "Float".into(),
        TypeRef::Number => "Number".into(),
        TypeRef::String => "String".into(),
        TypeRef::Bool => "Bool".into(),
        TypeRef::None => "None".into(),
        TypeRef::List(None) => "List".into(),
        TypeRef::List(Some(x)) => format!("List[{}]", type_ref_name(x)),
    }
}
fn numbers(a: Value, b: Value, s: Span, e: &Evaluator) -> Result<(f64, f64), Diagnostic> {
    Ok((as_number(&a, s, e)?, as_number(&b, s, e)?))
}
fn as_number(v: &Value, s: Span, e: &Evaluator) -> Result<f64, Diagnostic> {
    match v {
        Value::Int(x) => Ok(*x as f64),
        Value::Float(x) => Ok(*x),
        v => Err(e.type_error(s, "a Number", v)),
    }
}
fn as_int(v: Value, s: Span, e: &Evaluator) -> Result<i64, Diagnostic> {
    if let Value::Int(x) = v {
        Ok(x)
    } else {
        Err(e.type_error(s, "an Int", &v))
    }
}
fn as_string<'a>(v: &'a Value, s: Span, e: &Evaluator) -> Result<&'a str, Diagnostic> {
    if let Value::String(x) = v {
        Ok(x)
    } else {
        Err(e.type_error(s, "a String", v))
    }
}
fn numeric2(
    a: Value,
    b: Value,
    s: Span,
    e: &Evaluator,
    float: impl Fn(f64, f64) -> f64,
    int: impl Fn(i64, i64) -> Option<i64>,
) -> Result<Value, Diagnostic> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => int(x, y).map(Value::Int).ok_or_else(|| {
            e.error(
                "Value Error",
                "integer overflow",
                s,
                "This result is outside Int limits.",
                "Use Float values or smaller numbers.",
            )
        }),
        (a, b) => {
            let (x, y) = numbers(a, b, s, e)?;
            Ok(Value::Float(float(x, y)))
        }
    }
}
fn compare(a: &Value, b: &Value) -> Option<Ordering> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x.partial_cmp(y),
        (Value::Float(x), Value::Float(y)) => x.partial_cmp(y),
        (Value::Int(x), Value::Float(y)) => (*x as f64).partial_cmp(y),
        (Value::Float(x), Value::Int(y)) => x.partial_cmp(&(*y as f64)),
        (Value::String(x), Value::String(y)) => Some(x.cmp(y)),
        (Value::Bool(x), Value::Bool(y)) => Some(x.cmp(y)),
        _ => None,
    }
}
fn normalize_index(i: i64, len: usize) -> Option<usize> {
    let n = if i < 0 { len as i64 + i } else { i };
    if n >= 0 && (n as usize) < len {
        Some(n as usize)
    } else {
        None
    }
}
fn slice_bounds(
    a: Option<i64>,
    z: Option<i64>,
    len: usize,
    s: Span,
    e: &Evaluator,
) -> Result<(usize, usize), Diagnostic> {
    let norm = |x: i64| {
        if x < 0 {
            (len as i64 + x).max(0) as usize
        } else {
            (x as usize).min(len)
        }
    };
    let a = a.map(norm).unwrap_or(0);
    let z = z.map(norm).unwrap_or(len);
    if a > z {
        Err(e.error(
            "Index Error",
            "slice start is after its end",
            s,
            "This MVP supports forward slices only.",
            "Use a start index less than or equal to the end.",
        ))
    } else {
        Ok((a, z))
    }
}
fn repeat_string(s: String, n: i64, span: Span, e: &Evaluator) -> Result<Value, Diagnostic> {
    if n < 0 {
        return Err(e.error(
            "Value Error",
            "repeat count cannot be negative",
            span,
            "Repetition needs a non-negative count.",
            "Use zero or a positive Int.",
        ));
    }
    Ok(Value::String(s.repeat(n as usize)))
}
fn repeat_list(
    v: Rc<RefCell<Vec<Value>>>,
    n: i64,
    span: Span,
    e: &Evaluator,
) -> Result<Value, Diagnostic> {
    if n < 0 {
        return Err(e.error(
            "Value Error",
            "repeat count cannot be negative",
            span,
            "Repetition needs a non-negative count.",
            "Use zero or a positive Int.",
        ));
    }
    let original = v.borrow();
    let mut repeated = Vec::with_capacity(original.len().saturating_mul(n as usize));
    for _ in 0..n {
        repeated.extend(original.iter().cloned());
    }
    Ok(Value::List(Rc::new(RefCell::new(repeated)), false))
}
fn zero_args(a: &[Value], s: Span, n: &str, e: &Evaluator) -> Result<(), Diagnostic> {
    if a.is_empty() {
        Ok(())
    } else {
        Err(e.arg_error(s, n, 0, a.len()))
    }
}
fn one_arg(a: &[Value], s: Span, n: &str, e: &Evaluator) -> Result<(), Diagnostic> {
    if a.len() == 1 {
        Ok(())
    } else {
        Err(e.arg_error(s, n, 1, a.len()))
    }
}
fn aggregate(n: &str, v: &[Value], s: Span, e: &Evaluator) -> Result<Value, Diagnostic> {
    if v.is_empty() && !matches!(n, "sum" | "product") {
        return Err(e.error(
            "Value Error",
            format!("{n} requires a non-empty List"),
            s,
            "There is no aggregate value for an empty list.",
            "Add numeric values first.",
        ));
    }
    let nums = v
        .iter()
        .map(|x| as_number(x, s, e))
        .collect::<Result<Vec<_>, _>>()?;
    match n {
        "sum" => {
            if v.iter().all(|x| matches!(x, Value::Int(_))) {
                let mut total = 0i64;
                for value in v {
                    let Value::Int(value) = value else {
                        unreachable!()
                    };
                    total = total.checked_add(*value).ok_or_else(|| {
                        e.error(
                            "Value Error",
                            "integer overflow in sum",
                            s,
                            "The sum exceeds Int limits.",
                            "Use Float values or smaller numbers.",
                        )
                    })?;
                }
                Ok(Value::Int(total))
            } else {
                Ok(Value::Float(nums.iter().sum()))
            }
        }
        "product" => {
            if v.iter().all(|x| matches!(x, Value::Int(_))) {
                let mut total = 1i64;
                for value in v {
                    let Value::Int(value) = value else {
                        unreachable!()
                    };
                    total = total.checked_mul(*value).ok_or_else(|| {
                        e.error(
                            "Value Error",
                            "integer overflow in product",
                            s,
                            "The product exceeds Int limits.",
                            "Use Float values or smaller numbers.",
                        )
                    })?;
                }
                Ok(Value::Int(total))
            } else {
                Ok(Value::Float(nums.iter().product()))
            }
        }
        "mean" => Ok(Value::Float(nums.iter().sum::<f64>() / nums.len() as f64)),
        "median" => {
            let mut sorted = nums.clone();
            sorted.sort_by(f64::total_cmp);
            let middle = sorted.len() / 2;
            let value = if sorted.len() % 2 == 0 {
                (sorted[middle - 1] + sorted[middle]) / 2.0
            } else {
                sorted[middle]
            };
            Ok(Value::Float(value))
        }
        "variance" | "std" => {
            let mean = nums.iter().sum::<f64>() / nums.len() as f64;
            let variance =
                nums.iter().map(|value| (value - mean).powi(2)).sum::<f64>() / nums.len() as f64;
            Ok(Value::Float(if n == "std" {
                variance.sqrt()
            } else {
                variance
            }))
        }
        "mode" => {
            let mut sorted = nums.clone();
            sorted.sort_by(f64::total_cmp);
            let mut best = sorted[0];
            let mut best_count = 1;
            let mut current = sorted[0];
            let mut current_count = 1;
            for value in sorted.into_iter().skip(1) {
                if value.total_cmp(&current) == Ordering::Equal {
                    current_count += 1;
                } else {
                    if current_count > best_count {
                        best = current;
                        best_count = current_count;
                    }
                    current = value;
                    current_count = 1;
                }
            }
            if current_count > best_count {
                best = current;
            }
            Ok(Value::Float(best))
        }
        "min" => Ok(v[nums
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(Ordering::Equal))
            .unwrap()
            .0]
            .clone()),
        "max" => Ok(v[nums
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(Ordering::Equal))
            .unwrap()
            .0]
            .clone()),
        _ => unreachable!(),
    }
}
fn unary_math(
    n: &str,
    a: Vec<Value>,
    s: Span,
    e: &Evaluator,
    f: impl Fn(f64) -> f64,
) -> Result<Value, Diagnostic> {
    one_arg(&a, s, n, e)?;
    let out = f(as_number(&a[0], s, e)?);
    if out.is_nan() {
        Err(e.error(
            "Math Error",
            format!("{n} produced an invalid result"),
            s,
            "The input is outside the real-number domain.",
            "Use a value in the function's valid domain.",
        ))
    } else {
        Ok(Value::Float(out))
    }
}

fn binary_math(
    n: &str,
    a: Vec<Value>,
    s: Span,
    e: &Evaluator,
    f: impl Fn(f64, f64) -> f64,
) -> Result<Value, Diagnostic> {
    if a.len() != 2 {
        return Err(e.arg_error(s, n, 2, a.len()));
    }
    let out = f(as_number(&a[0], s, e)?, as_number(&a[1], s, e)?);
    if out.is_nan() {
        Err(e.error(
            "Math Error",
            format!("{n} produced an invalid result"),
            s,
            "The inputs are outside the real-number domain.",
            "Use values in the function's valid domain.",
        ))
    } else {
        Ok(Value::Float(out))
    }
}

fn integer_pair_math(
    n: &str,
    left: i64,
    right: i64,
    s: Span,
    e: &Evaluator,
) -> Result<Value, Diagnostic> {
    let mut a = left.unsigned_abs();
    let mut b = right.unsigned_abs();
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    let result = if n == "gcd" {
        a
    } else if left == 0 || right == 0 {
        0
    } else {
        left.unsigned_abs()
            .checked_div(a)
            .and_then(|value| value.checked_mul(right.unsigned_abs()))
            .ok_or_else(|| {
                e.error(
                    "Value Error",
                    "integer overflow in lcm",
                    s,
                    "The least common multiple exceeds Int limits.",
                    "Use smaller integers.",
                )
            })?
    };
    i64::try_from(result).map(Value::Int).map_err(|_| {
        e.error(
            "Value Error",
            format!("{n} result exceeds Int limits"),
            s,
            "The result cannot be represented as Int.",
            "Use smaller integers.",
        )
    })
}
