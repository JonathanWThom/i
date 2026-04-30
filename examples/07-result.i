module Main
    expose main

use Std.Float as F
use Std.IO (print)

type ParseError
    BadNumber
    OutOfRange

bounded = s lo hi ->
    n = F.parse s?
    n < lo or n > hi match
        True   -> Error OutOfRange
        False  -> Ok n

main =
    bounded "42", 0, 100 match
        Ok n      -> print! "got " ++ show n
        Error _   -> print! "bad input"
