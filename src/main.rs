use std::{
    fs::{File, OpenOptions},
    io::Read,
    time::Instant,
};

use editerra::{
    error::MapEdiError,
    expr_engine::ExprEngine,
    functions::register_functions_default,
    map_doc::{Delimiters, EdiMapDoc},
    map_edi,
};

fn main() -> Result<(), MapEdiError> {
    let mut handle = OpenOptions::new()
        .read(true)
        .open("./test.edi-map.yml")
        .expect("read map should work");
    let mut buf = String::new();

    handle.read_to_string(&mut buf).expect("file should read");

    let edi_map_doc =
        serde_yaml::from_str::<EdiMapDoc>(&buf).expect("edi map doc should deserialize");
    let mut handle = File::open("./batch.json").expect("read should work");
    let mut buf = String::new();
    handle.read_to_string(&mut buf).expect("file should read");
    let mut expr_engine = ExprEngine::default();

    expr_engine
        .init_with_haystack(buf)
        .expect("it should parse");

    expr_engine
        .register_functions(register_functions_default)
        .expect("it should reg funcs");

    let start = Instant::now();
    let result = map_edi::map_edi(edi_map_doc, &Delimiters::default(), &mut expr_engine)?;
    println!("{}", result.unwrap());
    println!("{:?}", start.elapsed());

    Ok(())
}
