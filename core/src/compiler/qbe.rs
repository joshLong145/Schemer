//! QBE Intermediate Representation
//!
//! Data structures for representing QBE IR and writing it to text format.

use std::fmt::Write;

/// QBE types
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QbeType {
    Word,   // w - 32-bit
    Long,   // l - 64-bit
    Single, // s - 32-bit float
    Double, // d - 64-bit float
    Byte,   // b - 8-bit
    Half,   // h - 16-bit
}

impl QbeType {
    pub fn to_qbe(&self) -> &'static str {
        match self {
            QbeType::Word => "w",
            QbeType::Long => "l",
            QbeType::Single => "s",
            QbeType::Double => "d",
            QbeType::Byte => "b",
            QbeType::Half => "h",
        }
    }
}

/// QBE values (operands)
#[derive(Clone, Debug)]
pub enum QbeValue {
    /// Temporary: %name
    Temp(String),
    /// Global symbol: $name
    Global(String),
    /// Integer constant
    Const(i64),
    /// Floating point constant
    Float(f64),
}

impl QbeValue {
    pub fn write(&self, out: &mut String) {
        match self {
            QbeValue::Temp(name) => write!(out, "%{}", name).unwrap(),
            QbeValue::Global(name) => write!(out, "${}", name).unwrap(),
            QbeValue::Const(n) => write!(out, "{}", n).unwrap(),
            QbeValue::Float(f) => write!(out, "d_{}", f.to_bits()).unwrap(),
        }
    }
}

/// QBE instructions
#[derive(Clone, Debug)]
pub enum QbeInst {
    /// Assignment: %dest =t op args
    Assign {
        dest: String,
        ty: QbeType,
        op: QbeOp,
    },

    /// Conditional jump: jnz %cond, @true, @false
    Jnz {
        cond: QbeValue,
        if_true: String,
        if_false: String,
    },

    /// Unconditional jump: jmp @label
    Jmp(String),

    /// Return: ret %value
    Ret(Option<QbeValue>),

    /// Call: %dest =l call $func(args...)
    Call {
        dest: Option<String>,
        func: QbeValue,
        args: Vec<(QbeType, QbeValue)>,
    },

    /// Store: storel %val, %addr
    Store {
        ty: QbeType,
        value: QbeValue,
        addr: QbeValue,
    },

    /// Load: %dest =l loadl %addr
    Load {
        dest: String,
        ty: QbeType,
        addr: QbeValue,
    },

    /// Halt (calls abort)
    Hlt,
}

impl QbeInst {
    pub fn write(&self, out: &mut String) {
        match self {
            QbeInst::Assign { dest, ty, op } => {
                write!(out, "%{} ={} ", dest, ty.to_qbe()).unwrap();
                op.write(out);
            }

            QbeInst::Jnz {
                cond,
                if_true,
                if_false,
            } => {
                out.push_str("jnz ");
                cond.write(out);
                write!(out, ", @{}, @{}", if_true, if_false).unwrap();
            }

            QbeInst::Jmp(label) => {
                write!(out, "jmp @{}", label).unwrap();
            }

            QbeInst::Ret(val) => {
                out.push_str("ret");
                if let Some(v) = val {
                    out.push(' ');
                    v.write(out);
                }
            }

            QbeInst::Call { dest, func, args } => {
                if let Some(d) = dest {
                    write!(out, "%{} =l ", d).unwrap();
                }
                out.push_str("call ");
                func.write(out);
                out.push('(');
                for (i, (ty, val)) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    write!(out, "{} ", ty.to_qbe()).unwrap();
                    val.write(out);
                }
                out.push(')');
            }

            QbeInst::Store { ty, value, addr } => {
                write!(out, "store{} ", ty.to_qbe()).unwrap();
                value.write(out);
                out.push_str(", ");
                addr.write(out);
            }

            QbeInst::Load { dest, ty, addr } => {
                write!(out, "%{} ={} load{} ", dest, ty.to_qbe(), ty.to_qbe()).unwrap();
                addr.write(out);
            }

            QbeInst::Hlt => {
                out.push_str("hlt");
            }
        }
    }
}

/// QBE operations
#[derive(Clone, Debug)]
pub enum QbeOp {
    // Arithmetic
    Add(QbeValue, QbeValue),
    Sub(QbeValue, QbeValue),
    Mul(QbeValue, QbeValue),
    Div(QbeValue, QbeValue),
    Rem(QbeValue, QbeValue),

    // Bitwise
    And(QbeValue, QbeValue),
    Or(QbeValue, QbeValue),
    Xor(QbeValue, QbeValue),
    Sar(QbeValue, QbeValue), // Arithmetic shift right
    Shl(QbeValue, QbeValue), // Shift left
    Shr(QbeValue, QbeValue), // Logical shift right

    // Comparison (returns 0 or 1)
    Ceql(QbeValue, QbeValue),  // Equal (long)
    Cnel(QbeValue, QbeValue),  // Not equal
    Csltl(QbeValue, QbeValue), // Signed less than
    Csgtl(QbeValue, QbeValue), // Signed greater than
    Cslel(QbeValue, QbeValue), // Signed less or equal
    Csgel(QbeValue, QbeValue), // Signed greater or equal

    // Memory/Copy
    Copy(QbeValue),

    // Extensions
    Extuw(QbeValue), // Zero-extend unsigned word to long
    Extsw(QbeValue), // Sign-extend signed word to long

    // Call (when used in assignment context)
    Call {
        func: QbeValue,
        args: Vec<(QbeType, QbeValue)>,
    },
}

impl QbeOp {
    pub fn write(&self, out: &mut String) {
        match self {
            QbeOp::Add(a, b) => {
                out.push_str("add ");
                a.write(out);
                out.push_str(", ");
                b.write(out);
            }
            QbeOp::Sub(a, b) => {
                out.push_str("sub ");
                a.write(out);
                out.push_str(", ");
                b.write(out);
            }
            QbeOp::Mul(a, b) => {
                out.push_str("mul ");
                a.write(out);
                out.push_str(", ");
                b.write(out);
            }
            QbeOp::Div(a, b) => {
                out.push_str("div ");
                a.write(out);
                out.push_str(", ");
                b.write(out);
            }
            QbeOp::Rem(a, b) => {
                out.push_str("rem ");
                a.write(out);
                out.push_str(", ");
                b.write(out);
            }
            QbeOp::And(a, b) => {
                out.push_str("and ");
                a.write(out);
                out.push_str(", ");
                b.write(out);
            }
            QbeOp::Or(a, b) => {
                out.push_str("or ");
                a.write(out);
                out.push_str(", ");
                b.write(out);
            }
            QbeOp::Xor(a, b) => {
                out.push_str("xor ");
                a.write(out);
                out.push_str(", ");
                b.write(out);
            }
            QbeOp::Sar(a, b) => {
                out.push_str("sar ");
                a.write(out);
                out.push_str(", ");
                b.write(out);
            }
            QbeOp::Shl(a, b) => {
                out.push_str("shl ");
                a.write(out);
                out.push_str(", ");
                b.write(out);
            }
            QbeOp::Shr(a, b) => {
                out.push_str("shr ");
                a.write(out);
                out.push_str(", ");
                b.write(out);
            }
            QbeOp::Ceql(a, b) => {
                out.push_str("ceql ");
                a.write(out);
                out.push_str(", ");
                b.write(out);
            }
            QbeOp::Cnel(a, b) => {
                out.push_str("cnel ");
                a.write(out);
                out.push_str(", ");
                b.write(out);
            }
            QbeOp::Csltl(a, b) => {
                out.push_str("csltl ");
                a.write(out);
                out.push_str(", ");
                b.write(out);
            }
            QbeOp::Csgtl(a, b) => {
                out.push_str("csgtl ");
                a.write(out);
                out.push_str(", ");
                b.write(out);
            }
            QbeOp::Cslel(a, b) => {
                out.push_str("cslel ");
                a.write(out);
                out.push_str(", ");
                b.write(out);
            }
            QbeOp::Csgel(a, b) => {
                out.push_str("csgel ");
                a.write(out);
                out.push_str(", ");
                b.write(out);
            }
            QbeOp::Copy(v) => {
                out.push_str("copy ");
                v.write(out);
            }
            QbeOp::Extuw(v) => {
                out.push_str("extuw ");
                v.write(out);
            }
            QbeOp::Extsw(v) => {
                out.push_str("extsw ");
                v.write(out);
            }
            QbeOp::Call { func, args } => {
                out.push_str("call ");
                func.write(out);
                out.push('(');
                for (i, (ty, val)) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    write!(out, "{} ", ty.to_qbe()).unwrap();
                    val.write(out);
                }
                out.push(')');
            }
        }
    }
}

/// QBE basic block
#[derive(Clone, Debug)]
pub struct QbeBlock {
    pub label: String,
    pub instructions: Vec<QbeInst>,
}

impl QbeBlock {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            instructions: Vec::new(),
        }
    }

    pub fn write(&self, out: &mut String) {
        writeln!(out, "@{}", self.label).unwrap();
        for inst in &self.instructions {
            out.push_str("    ");
            inst.write(out);
            out.push('\n');
        }
    }
}

/// QBE function definition
#[derive(Clone, Debug)]
pub struct QbeFunction {
    pub export: bool,
    pub name: String,
    pub params: Vec<(QbeType, String)>,
    pub return_type: Option<QbeType>,
    pub blocks: Vec<QbeBlock>,
}

impl QbeFunction {
    pub fn write(&self, out: &mut String) {
        if self.export {
            out.push_str("export ");
        }

        out.push_str("function ");

        if let Some(ref ty) = self.return_type {
            write!(out, "{} ", ty.to_qbe()).unwrap();
        }

        write!(out, "${}(", self.name).unwrap();

        for (i, (ty, name)) in self.params.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            write!(out, "{} %{}", ty.to_qbe(), name).unwrap();
        }

        out.push_str(") {\n");

        for block in &self.blocks {
            block.write(out);
        }

        out.push_str("}\n");
    }
}

/// QBE data definition
#[derive(Clone, Debug)]
pub struct QbeData {
    pub export: bool,
    pub name: String,
    pub items: Vec<QbeDataItem>,
}

#[derive(Clone, Debug)]
pub enum QbeDataItem {
    String(String),
    Long(i64),
    Word(i32),
    Byte(u8),
    Zero(usize),
}

impl QbeData {
    pub fn write(&self, out: &mut String) {
        if self.export {
            out.push_str("export ");
        }

        write!(out, "data ${} = {{ ", self.name).unwrap();

        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            match item {
                QbeDataItem::String(s) => {
                    write!(out, "b \"{}\"", escape_string(s)).unwrap();
                }
                QbeDataItem::Long(n) => {
                    write!(out, "l {}", n).unwrap();
                }
                QbeDataItem::Word(n) => {
                    write!(out, "w {}", n).unwrap();
                }
                QbeDataItem::Byte(n) => {
                    write!(out, "b {}", n).unwrap();
                }
                QbeDataItem::Zero(n) => {
                    write!(out, "z {}", n).unwrap();
                }
            }
        }

        out.push_str(" }\n");
    }
}

/// Complete QBE module
#[derive(Clone, Debug, Default)]
pub struct QbeModule {
    pub functions: Vec<QbeFunction>,
    pub data: Vec<QbeData>,
}

impl QbeModule {
    pub fn new() -> Self {
        Self::default()
    }

    /// Render the module to QBE IR text format
    pub fn render(&self) -> String {
        let mut out = String::new();

        // Write data section first
        for data in &self.data {
            data.write(&mut out);
        }

        if !self.data.is_empty() {
            out.push('\n');
        }

        // Write functions
        for func in &self.functions {
            func.write(&mut out);
            out.push('\n');
        }

        out
    }
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
        .replace('\r', "\\r")
}
