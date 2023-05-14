use fork::{daemon, Fork};
use std::thread;
use std::time::Duration;

fn main() {
    let child_pid = match daemon(false, false) {
        Ok(Fork::Child) => daemon_f(),
        Ok(Fork::Parent(child_pid)) => {
            println!("This arm is not entered?");
            child_pid
        }
        Err(e) => panic!("error"),
        // daemon_f();
    };

    println!("Child has pid ({})", child_pid);
}

fn daemon_f() -> ! {
    loop {
        println!("should go into the void");
        thread::sleep(Duration::from_secs(1));
    }
}
