module Main
    expose main

use Std.IO (print, println)

# Pure callback — `map` stays pure.
double = n -> n * 2

doubled = [1, 2, 3].map double

# Effectful callback — `map` inherits ! IO.
shout = n ->
    println! "saw " ++ show n
    n

main =
    [4, 5, 6].map shout              # ! IO inferred, runs each side effect
    println! "doubled: " ++ show doubled
