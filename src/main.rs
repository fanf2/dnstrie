use dnstrie::trieprep::BYTE_TO_BITS;

fn main() {
    for (byte, &bits) in BYTE_TO_BITS.iter().enumerate() {
        if bits < 256 {
            print!("  {:02x}", bits);
        } else {
            print!("{:04x}", bits);
        }
        if byte & 15 == 15 {
            println!();
        } else {
            print!(" ")
        }
    }
}
