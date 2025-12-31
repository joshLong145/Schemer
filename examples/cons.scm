
(begin
    (define a (cons 1 (cons 2 (cons 3 '()))))
    (define b (cons (+ 2 2) (cons 3 (cons 2 1))))
    (display a)
    (display b)


    (cond
        ((> 1 2) #f)
        ((> 1 5) #f)
        ((> 4 5) #t)
        (else '(1 2 3))
    )
)
