(begin 
    (define r 10)
    (define f (lambda (x) (* x x)))
    ( define s 
        (if (number? f) (+ 2 2) (print (0)))
    )

    (print s)
)
