module Main
    expose main

type Tree a
    Leaf
    Node
        value : a
        left : Tree a
        right : Tree a

count = tree ->
    tree match
        Leaf            -> 0
        Node v, l, r    -> 1 + count l + count r

main =
    t = Node(value = 1, left = Leaf, right = Node(value = 2, left = Leaf, right = Leaf))
    print! "count: " ++ show (count t)
