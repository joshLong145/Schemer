(begin
    (define a '(1 10 3))
    (define b (map (lambda (x) (
        if (< 2 x) (+ x 1) (+ x 2)))
    a))

    (display b)

    (define c (filter (lambda (x) (
        if (< 2 x) #t #f))
    a))

    (display c)
)
