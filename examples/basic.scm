(begin
    (define foo (lambda (x) (+ 1 x)))
    (define a (map (lambda (x) (
        if (< 2 (foo x)) (+ x 1) (+ x 2)))
    (1 10 3)))

    (define fib (lambda (x) (
        if (< x 2) 1 (+ (fib (- x 1)) (fib (- x 2)))
    )))

    (display (fib 20))

    (display (eval a))

    (define test '(a 2 3))
    (display (eval test))
    (display (list 1 2 3))
)
