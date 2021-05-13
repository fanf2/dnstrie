mod bmpvec;
mod dname;

fn main() {
    for byte in 0..=255 {
        let bits = dname::BYTE_TO_BITS[byte];
        if bits < 256 {
            print!("   {:02x}", bits);
        } else {
            print!(" {:04x}", bits);
        }
        if byte & 7 == 7 {
            print!("\n");
        }
    }
}
