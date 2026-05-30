type Point
    x : Int
    y : Int
impl Eq Point
    eq = a b -> a.x == b.x
    ne = a b -> not (a.x == b.x)
samePoint = a b -> a == b
p = Point(x = 1, y = 2)
pointsEq = p == p
