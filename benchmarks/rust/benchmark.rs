const ROUNDS: i64 = 2;
const INTEGER_ITERATIONS: i64 = 750_000;
const FLOAT_ITERATIONS: i64 = 150_000;
const LIST_SIZE: i64 = 100_000;

fn integer_work() -> i64 {
    let mut state = 1_i64;
    let mut checksum = 0_i64;
    for i in 0..INTEGER_ITERATIONS {
        state = (state * 1_664_525 + 1_013_904_223 + i) % 2_147_483_647;
        checksum = (checksum + state) % 9_223_372_036_854_775_000;
    }
    checksum
}

fn floating_work() -> f64 {
    let mut checksum = 0.0_f64;
    for i in 0..FLOAT_ITERATIONS {
        let x = (i + 1) as f64 * 0.00001;
        checksum += x.sin() * x.cos() + (x + 1.0).sqrt() + (x + 1.0).ln();
    }
    checksum
}

fn list_work() -> i64 {
    let mut values = Vec::new();
    let mut state = 7_i64;
    for i in 0..LIST_SIZE {
        state = (state * 48_271 + i) % 2_147_483_647;
        values.push(state);
    }
    values.sort_unstable();
    let middle = (LIST_SIZE / 2) as usize;
    values[0] + values[middle] + values[LIST_SIZE as usize - 1] + values.len() as i64
}

fn main() {
    let mut integer_checksum = 0_i64;
    let mut floating_checksum = 0.0_f64;
    let mut list_checksum = 0_i64;

    for round_index in 0..ROUNDS {
        integer_checksum += integer_work() + round_index;
        floating_checksum += floating_work();
        list_checksum += list_work();
    }

    println!("integer={integer_checksum}");
    println!("float={floating_checksum:.6}");
    println!("list={list_checksum}");
}
