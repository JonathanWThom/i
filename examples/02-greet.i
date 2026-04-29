module Main
    expose main

use Std.IO (print, readLine)

main =
    print! "what's your name?"
    name = readLine!
    print! "hi, " ++ name
