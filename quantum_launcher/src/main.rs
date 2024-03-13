use quantum_launcher_backend as backend;

fn main() {
    println!("Trying 1.20.4");
    match backend::instance::create("test_instance", "1.20.4".to_owned()) {
        Ok(_) => {}
        Err(err) => {
            println!("Failed with error: {:?}", err)
        }
    };
}
