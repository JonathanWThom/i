module M
    expose make

type Float
    v : Float

type Shape
    Circle
        radius : Float
    Rect
        width : Float
        height : Float

make = Circle(radius = 1.0)
