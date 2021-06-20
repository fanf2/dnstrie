use core::convert::TryFrom;
use dnstrie::error::*;
use dnstrie::*;

fn main() -> Result<()> {
    let text = "dotat.at";
    let name = HeapName::try_from(text)?;
    println!("{}", text);
    println!("{}", name);
    println!("{:#?}", name);
    let mut key = TrieName::new();
    key.from_dns_name(&name);
    println!("{:#?}", key);
    Ok(())
}
