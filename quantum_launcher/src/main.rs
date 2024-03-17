use quantum_launcher_backend as backend;

fn main() {
    // println!("Trying 1.20.4");
    // match backend::instance::create("test_instance2", "1.20.4".to_owned()) {
    //     Ok(_) => {}
    //     Err(err) => {
    //         println!("Failed with error: {:?}", err)
    //     }
    // };
    // backend::instance::create("test_instance2", "1.20.4".to_owned()).unwrap();
    // backend::java_locate::JavaInstall::find_java_installs()
    //     .unwrap()
    //     .iter()
    //     .for_each(|n| println!("{:?}", n));
    backend::instance::launch("test_instance2", "mrmayman", "2G").unwrap();
}
