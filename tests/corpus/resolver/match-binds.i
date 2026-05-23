module M
    expose f

type Option a
    Some : a
    None

f = o ->
    o match
        Some y -> y
        None -> 0
