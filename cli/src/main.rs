fn main() {
    let script = "print(fetch(\"www.google.dk\"));";
    lib::init_rhai_engine(script).unwrap();
}
