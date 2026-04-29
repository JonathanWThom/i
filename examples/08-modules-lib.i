module Geometry
    expose Point, distance

type Point
    x : Float
    y : Float

distance = a, b ->
    ((a.x - b.x)^2 + (a.y - b.y)^2)^0.5
