//! Compares platforms' traces of one day (`crosscheck --trace-day D`): for
//! each, the first tick and sections where it parts from the first.
//!
//!   cargo run --release -p rim_sim --example traces -- trace-*.txt
//!
//! Exits 1 when any trace parts from the first.

fn main() {
    let files: Vec<String> = std::env::args().skip(1).collect();
    let read = |f: &String| std::fs::read_to_string(f).unwrap_or_else(|e| panic!("{f}: {e}"));
    let Some((first, rest)) = files.split_first() else {
        eprintln!("usage: traces TRACE TRACE...");
        std::process::exit(2);
    };
    let base = read(first);
    let mut parted = false;
    for f in rest {
        match rim_sim::bisect::traces(&base, &read(f)) {
            None => println!("{f} agrees with {first} at every tick"),
            Some(p) => {
                parted = true;
                println!("{f} parts from {first} at tick {}: {}", p.tick, p.sections.join(", "));
            }
        }
    }
    std::process::exit(i32::from(parted));
}
