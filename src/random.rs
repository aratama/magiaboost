pub fn random_select_mut<T: Copy>(xs: &mut Vec<T>) -> T {
    xs.remove((rand::random::<usize>() % xs.len()) as usize)
}

pub fn random_select<T: Copy>(xs: &Vec<T>) -> &T {
    xs.get((rand::random::<usize>() % xs.len()) as usize)
        .unwrap()
}