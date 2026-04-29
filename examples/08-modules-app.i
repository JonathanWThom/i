module Main
    expose main

use Geometry (Point, distance)
use Std.IO (print)

main =
    p1 = Point(x = 0, y = 0)
    p2 = Point(x = 3, y = 4)
    print! "distance: " ++ show (distance p1, p2)
