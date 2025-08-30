(begin
    (define a (map (lambda (x) (
        if (< 2 x) (+ x 1) (+ x 2)))
    (1 10 3)))

    (display a)
)
