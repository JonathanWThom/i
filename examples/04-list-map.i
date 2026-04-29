module Main
    expose main

main =
    nums = [1, 2, 3, 4, 5]
    doubled = nums.map x -> x * 2
    print! "doubled: " ++ show doubled
