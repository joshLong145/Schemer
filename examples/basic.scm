(begin
    (define foo (lambda (x) (+ 1 x)))
    (define a (map (lambda (x) (
        if (< 2 (foo x)) (+ x 1) (+ x 2)))
    (1 10 3)))

    (print a)

    (define fib (lambda (x) (
        if (< x 2) 1 (+ (fib (- x 1)) (fib (- x 2)))
    )))

    (print (fib 1))

    (define f (map (lambda (x) (begin
        (define bar (begin (foo x)))
        (+ bar 1)
    ))
    (9)))

    (print f)
)
