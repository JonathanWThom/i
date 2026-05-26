type Maybe a
    None
    Some : a

unwrap = m -> m match
    Some n -> n
    None -> 0
