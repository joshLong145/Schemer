
(begin
    (define a (cons 1 (cons 2 (cons 3 '()))))
    (define b (cons (+ 2 2) (cons 3 (cons 2 1))))
    (display a)
    (newline)
    (display b)
    (newline)

    (display (cond
        ((> 1 9) #t)
        ((> 1 5) #f)
        ((> 4 5) #t)
        (else '(1 2 3))
    ))
)
