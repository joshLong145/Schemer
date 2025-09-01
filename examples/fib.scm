(begin
    (define fib (lambda (x) (
        if (< x 2)
            1
            (+ (fib (- x 1)) (fib (- x 2)))
    )))

    (display (fib 14))
)
