extern crate smallvec;

use smallvec::SmallVec;

fn main()
{
    println!("Hello world!");
    let mut v = SmallVec::<[u8; 4]>::new(); // initialize an empty vector

    // The vector can hold up to 4 items without spilling onto the heap.
    v.extend(0..4);
    assert_eq!(v.len(), 4);
    assert!(!v.spilled());

    // Pushing another element will force the buffer to spill:
    v.push(4);
    assert_eq!(v.len(), 5);
    assert!(v.spilled());
    println!("Hello world again!");
}
