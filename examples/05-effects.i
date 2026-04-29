module Main
    expose main

# pure — Int -> Int
double = n -> n * 2

# pure — Int -> Int
quadruple = n -> double (double n)

# effectful — Int ! IO -> Unit
shout = n ->
    print! "the number is " ++ show n

# effectful — inferred ! IO
main =
    print! "starting"
    shout 21
    print! "done"
