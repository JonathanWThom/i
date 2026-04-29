module Main
    expose main

type Shape
    Circle
        radius : Float
    Rect
        width : Float
        height : Float

area = shape ->
    shape match
        Circle r    -> 3.14159 * r^2
        Rect w, h   -> w * h

main =
    s = Circle(radius = 5.0)
    print! "area: " ++ show (area s)
