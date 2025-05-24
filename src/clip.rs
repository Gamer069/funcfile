pub fn copy(val: String) {
    let mut board = copypasta_ext::try_context().expect("Failed to init clipboard");
    board.set_contents(val).expect("Failed to copy");
}

pub fn paste() -> String {
    let mut board = copypasta_ext::try_context().expect("Failed to init clipboard");
    board.get_contents().expect("Failed to paste")
}
