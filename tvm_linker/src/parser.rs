use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use tvm::stack::{BuilderData, IBitstring, IntegerData, SliceData};
use tvm::stack::integer::serialization::{Encoding, SignedIntegerBigEndianEncoding};
use tvm::stack::serialization::Serializer;
use tvm::stack::dictionary::{HashmapE, HashmapType};
use ton_block::*;

enum ObjectType {
    None,
    Function((u32, String)),
    Data(Vec<DataValue>),
}
impl From<&str> for ObjectType {
    fn from(stype: &str) -> ObjectType {
        match stype {
            "function" => ObjectType::Function((0, String::new())),
            "object" => ObjectType::Data(vec![]),
            _ => ObjectType::None,
        }
    }
}
enum DataValue {
    Number((IntegerData, usize)),
}

impl DataValue {
    pub fn write(&self) -> BuilderData {
        let mut b = BuilderData::new();
        match self {
            DataValue::Number(ref intgr) => {
                let encoding = SignedIntegerBigEndianEncoding::new(intgr.1);
                b.append_bitstring(&encoding.try_serialize(&intgr.0).unwrap().data()[..]).unwrap();
                b
            },
        }
    }
}

struct Object {
    pub name: String,
    pub size: usize,
    pub align: usize,
    pub dtype: ObjectType,
}

impl Object {
    pub fn new(name: String, stype: &str) -> Self {
        Object {
            name,
            size: 0,
            align: 0,
            dtype: ObjectType::from(stype),
        }
    }
}

impl Default for Object {
    fn default() -> Self {
        Object::new(String::new(), "")
    }
}

pub struct ParseEngine {
    xrefs: HashMap<String, u32>,
    intrefs: HashMap<String, i32>,
    aliases: HashMap<String, i32>,
    globals: HashMap<String, Object>,
    internals: HashMap<i32, String>,
    signed: HashMap<u32, bool>,
    entry_point: String,
}

const PATTERN_GLOBL:    &'static str = r"^[\t\s]*\.globl[\t\s]+([a-zA-Z0-9_]+)";
const PATTERN_DATA:     &'static str = r"^[\t\s]*\.data";
const PATTERN_INTERNAL: &'static str = r"^[\t\s]*\.internal[\t\s]+(:[a-zA-Z0-9_]+)";
const PATTERN_SELECTOR: &'static str = r"^[\t\s]*\.selector";
const PATTERN_ALIAS:    &'static str = r"^[\t\s]*\.internal-alias (:[a-zA-Z0-9_]+),[\t\s]+(-?\d+)";
const PATTERN_LABEL:    &'static str = r"^[a-zA-Z0-9_]+:";
const PATTERN_PARAM:    &'static str = r"^[\t\s]+\.([a-zA-Z0-9_]+)";
const PATTERN_TYPE:     &'static str = r"^[\t\s]*\.type[\t\s]+([a-zA-Z0-9_]+),[\t\s]*@([a-zA-Z]+)";
const PATTERN_SIZE:     &'static str = r"^[\t\s]*\.size[\t\s]+([a-zA-Z0-9_]+),[\t\s]*([\.a-zA-Z0-9_]+)";

const GLOBL:    &'static str = ".globl";
const INTERNAL: &'static str = ".internal";
const DATA:     &'static str = ".data";
const SELECTOR: &'static str = ".selector";

const FUNC_SUFFIX_AUTH: &'static str = "_authorized";

impl ParseEngine {

    pub fn new() -> Self {
        ParseEngine {
            xrefs:      HashMap::new(), 
            intrefs:    HashMap::new(), 
            aliases:    HashMap::new(),
            globals:   HashMap::new(), 
            internals:  HashMap::new(),
            signed:     HashMap::new(),
            entry_point: String::new(),
        }
    }

    pub fn parse<T: Read + Seek>(&mut self, source: T, libs: Vec<T>) -> Result<(), String> {
        for lib_buf in libs {
            let mut reader = BufReader::new(lib_buf);
            self.parse_code(&mut reader, true)?;
            reader.seek(SeekFrom::Start(0))
                .map_err(|e| format!("error while seeking lib file: {}", e))?;
            self.parse_code(&mut reader, false)?;
        }
        let mut reader = BufReader::new(source);
        self.parse_code(&mut reader, true)?;
        reader.seek(SeekFrom::Start(0))
            .map_err(|e| format!("error while seeking source file: {}", e))?;
        self.parse_code(&mut reader, false)?;

        if self.entry_point.is_empty() {
            return Err("Selector not found".to_string());
        }
        ok!()
    }

    pub fn data(&self) -> BuilderData {
        let mut data = BuilderData::new();
        data.append_reference(BuilderData::from_slice(&self.build_data()));
        data
    }

    pub fn entry(&self) -> &str {
        &self.entry_point
    }

    pub fn internals(&self) -> &HashMap<i32, String> {
        &self.internals
    }

    pub fn internal_by_name(&self, name: &str) -> Option<(i32, String)> {
        let id = self.intrefs.get(name)?;
        let body = self.internals.get(id).map(|v| v.to_owned())?;
        Some((*id, body))
    }

    pub fn globals(&self) -> HashMap<u32, String> {
        let mut funcs = HashMap::new();
        let iter = self.globals.iter().filter_map(|item| {
            match &item.1.dtype {
                ObjectType::Function(ref func) => Some(func),
                _ => None,
            }
        });
        for i in iter {
            funcs.insert(i.0, i.1.clone());
        }
        funcs
    }

    pub fn signed(&self) -> &HashMap<u32, bool> {
        &self.signed
    }

    fn parse_code<R: BufRead>(&mut self, reader: &mut R, first_pass: bool) -> Result<(), String> {
        let globl_regex = Regex::new(PATTERN_GLOBL).unwrap();
        let internal_regex = Regex::new(PATTERN_INTERNAL).unwrap();
        let selector_regex = Regex::new(PATTERN_SELECTOR).unwrap();
        let data_regex = Regex::new(PATTERN_DATA).unwrap();
        let label_regex = Regex::new(PATTERN_LABEL).unwrap();
        let dotted_regex = Regex::new(PATTERN_PARAM).unwrap();
        let alias_regex = Regex::new(PATTERN_ALIAS).unwrap();
        let type_regex = Regex::new(PATTERN_TYPE).unwrap();
        let size_regex = Regex::new(PATTERN_SIZE).unwrap();

        let mut section_name: String = String::new();
        let mut obj_body: String = "".to_owned();
        let mut obj_name: String = "".to_owned();
        let mut lnum = 0;
        let mut l = String::new();
        while reader.read_line(&mut l)
            .map_err(|_| "error while reading line")? != 0 {
            lnum += 1;
            if type_regex.is_match(&l) {
                let cap = type_regex.captures(&l).unwrap();
                let name = cap.get(1).unwrap().as_str().to_owned();
                let type_name = cap.get(2).unwrap().as_str().to_owned();
                let obj = self.globals.entry(name.clone()).or_insert(Object::new(name, &type_name));
                obj.dtype = ObjectType::from(type_name.as_str());
            } else if size_regex.is_match(&l) {
                let cap = size_regex.captures(&l).unwrap();
                let name = cap.get(1).unwrap().as_str().to_owned();
                let size_str = cap.get(2).ok_or(format!("line:{}: .size option is invalid", lnum))?.as_str().to_owned();
                let item_ref = self.globals.entry(name.clone()).or_insert(Object::new(name, ""));
                item_ref.size = usize::from_str_radix(&size_str, 10).unwrap_or(0);
            } else if globl_regex.is_match(&l) { 
                self.update(&section_name, &obj_name, &obj_body, first_pass)?;
                section_name = GLOBL.to_owned();
                obj_body = "".to_owned(); 
                obj_name = globl_regex.captures(&l).unwrap().get(1).unwrap().as_str().to_owned();
                self.globals.entry(obj_name.clone()).or_insert(Object::new(obj_name.clone(), ""));
            } else if data_regex.is_match(&l) {
                self.update(&section_name, &obj_name, &obj_body, first_pass)?;
                section_name = DATA.to_owned();
                obj_name = "".to_owned();
                obj_body = "".to_owned();
            } else if selector_regex.is_match(&l) {                
                self.update(&section_name, &obj_name, &obj_body, first_pass)?;
                if first_pass { 
                    section_name.clear();
                } else {
                    section_name = SELECTOR.to_owned();
                }
                obj_name = "".to_owned();
                obj_body = "".to_owned();
            } else if internal_regex.is_match(&l) {
                self.update(&section_name, &obj_name, &obj_body, first_pass)?;
                section_name = INTERNAL.to_owned();
                obj_body = "".to_owned();
                obj_name = internal_regex.captures(&l).unwrap().get(1).unwrap().as_str().to_owned();
            } else if alias_regex.is_match(&l) {
                let cap = alias_regex.captures(&l).unwrap();
                self.aliases.insert(
                    cap.get(1).unwrap().as_str().to_owned(), 
                    i32::from_str_radix(cap.get(2).unwrap().as_str(), 10)
                        .map_err(|_| format!("line: '{}': failed to parse id", l))?, 
                );                
            } else if label_regex.is_match(&l) { 
                
            } else if dotted_regex.is_match(&l) {
               obj_body.push_str(&l);
            } else {
                let l_with_numbers = if first_pass { l.to_owned() } else { self.replace_labels(&l) };
                obj_body.push_str(&l_with_numbers);
            }
            l.clear();
        }

        self.update(&section_name, &obj_name, &obj_body, first_pass)?;
        ok!()
    }

    fn update(&mut self, section: &str, func: &str, body: &str, first_pass: bool) -> Result<(), String> {
        match section {
            SELECTOR => {
                if self.entry_point.is_empty() {
                    self.entry_point = body.trim_end().to_string();
                } else {
                    return Err("Another selector found".to_string());
                }
            },
            GLOBL => {
               let item = self.globals.entry(func.to_owned()).or_default();
                match item.dtype {
                    ObjectType::Function(ref mut fparams) => {
                        let mut signed = false;
                        if let Some(index) = func.find(FUNC_SUFFIX_AUTH) {
                            if (index + FUNC_SUFFIX_AUTH.len()) == func.len() {
                                signed = true;
                            }
                        }
                        let func_id = calc_func_id(func);
                        fparams.0 = func_id;
                        fparams.1 = body.trim_end().to_string();
                        self.signed.insert(func_id, signed);
                        let prev = self.xrefs.insert(func.to_string(), func_id);
                        if first_pass && prev.is_some() {
                            Err(format!("global function with id = {} already exist", func_id))?;
                        }
                    },
                    ObjectType::Data(ref mut dparams) => {                        
                        Self::update_data(body, &mut item.size, dparams)?;
                    },
                    ObjectType::None => Err(format!("The type of global object {} is unknown. Use: .type {}, xxx", func, func))?,
                };
            },
            INTERNAL => {
                let f_id = self.aliases.get(func).ok_or(format!("id for '{}' not found", func))?;
                let prev = self.internals.insert(*f_id, body.trim_end().to_string());
                if first_pass && prev.is_some() {
                    Err(format!("internal function with id = {} already exist", *f_id))?;
                }
                self.intrefs.insert(func.to_string(), *f_id);
            },
            _ => (),
        }
        ok!()
    }

    fn update_data(body: &str, item_size: &mut usize, values: &mut Vec<DataValue>) -> Result<(), String> {
        lazy_static! {
            static ref PARAM_RE: Regex = Regex::new(PATTERN_PARAM).unwrap();
        }
        for param in body.lines() {
            if let Some(cap) = PARAM_RE.captures(param) {
                let value_len = match cap.get(1).unwrap().as_str() {
                    ".byte"  => 1,
                    ".long"  => 4,
                    ".short" => 2,
                    ".quad"  => 8,
                    _ => Err(format!("Unsupported parameter {}", cap.get(1).unwrap().as_str()))?,
                };
                if *item_size < value_len {
                    Err(format!("global object has invalid .size parameter: too small)"))?;
                }
                *item_size -= value_len;
                let value = param.get(cap.get(1).unwrap().start()..).unwrap().trim_start_matches(',').trim();
                values.push(DataValue::Number((
                    IntegerData::from_str_radix(value, 10).map_err(|_| "value is invalid number".to_string())?,
                    value_len,
                )));
            }
        }
        if *item_size > 0 {
            Err(format!("global object has invalid .size parameter: bigger than defined values"))?;
        }
        ok!()
    }

    fn build_data(&self) -> SliceData {
        let mut index = 0;
        let mut dict = HashmapE::with_bit_len(64);
        let iter = self.globals.iter().filter_map(|item| {
            match &item.1.dtype {
                ObjectType::Data(ref values) => Some(values),
                _ => None,
            }
        });

        for item in iter {
            for subitem in item {
                dict.set(
                    (index as u64).write_to_new_cell().unwrap().into(),
                    subitem.write().into()
                ).unwrap();
                index += 1;
            }
        }
        dict.get_data()
    }

    fn replace_labels(&mut self, line: &str) -> String {
        let mut result = String::new();
        let mut line = line;
        let re = Regex::new(r"\$:?[A-Za-z0-9_]+\$").unwrap();
        loop {
            line = match re.find(line) {
                None => {
                    result.push_str(line);
                    break result;
                }
                Some(mt) => {
                    let parts: Vec<&str> = re.split(line).collect();
                    result.push_str(parts.get(0).unwrap_or(&""));
                    let pointer = line.get(mt.start()+1..mt.end()-1).expect("failed to extract label from line");
                    let id_name = {
                        if pointer.starts_with(":") {
                            self.intrefs.get(pointer).map(|id| id.to_string())
                        } else {
                            self.xrefs.get(pointer).map(|id| id.to_string())
                        }
                    }.unwrap_or("???".to_string());
                    result.push_str(&id_name);
                    parts.get(1).unwrap_or(&"")
                }
            };
        }
    }

    pub fn debug_print(&self) {
        let line = "--------------------------";
        println!("Entry point:\n{}\n{}\n{}", line, self.entry(), line);
        println!("General-purpose functions:\n{}", line);
        for (k, v) in &self.xrefs {
            println! ("Function {:30}: id={:08X}, sign-check={:?}", k, v, self.signed.get(&v).unwrap());
        }
        for (k, v) in self.globals() {
            println! ("Function {:08X}\n{}\n{}\n{}", k, line, v, line);
        }        
        println!("{}\nInternal functions:\n{}", line, line);
        for (k, v) in &self.intrefs {
            println! ("Function {:30}: id={:08X}", k, v);
        }
        for (k, v) in &self.internals {
            println! ("Function {:08X}\n{}\n{}\n{}", k, line, v, line);
        }
    }
}

pub fn calc_func_id(func_interface: &str) -> u32 {
    let mut hasher = Sha256::new();
    hasher.input(func_interface.as_bytes());
    let mut id_bytes = [0u8; 4];
    id_bytes.copy_from_slice(&hasher.result()[..4]);
    u32::from_be_bytes(id_bytes)
} 


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn test_parser_testlib() {
        let mut parser = ParseEngine::new();
        let pbank_file = File::open("./tests/pbank.s").unwrap();
        let test_file = File::open("./tests/test.tvm").unwrap();
        assert_eq!(parser.parse(pbank_file, vec![test_file]), ok!());
        parser.debug_print();
    }

    //#[test]
    fn test_parser_stdlib() {
        let mut parser = ParseEngine::new();
        let pbank_file = File::open("./tests/pbank.s").unwrap();
        let test_file = File::open("./stdlib.tvm").unwrap();
        assert_eq!(parser.parse(pbank_file, vec![test_file]), ok!());
    }
}