(begin
    (define a (map (lambda (x) (if (< 1 x) (+ x 1) (+ x 2))) (1 2 3)))

    (define b 10)
    (define r (lambda (x) (+ x b)))
    (print (r (1)))

    (print a)

    (define fib (lambda (x) (
        if (< x 2) 0 (fib (- x 1))
    )))

    (fib (5))
)
