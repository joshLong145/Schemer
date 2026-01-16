(begin
    (define foo (1 2 3))
    (define gen-list (lambda (x y) (let
            (
                (a (car foo))
                (b (car (cdr foo)))
            )
            (list a b x y)
        )
    ))

    (gen-list 1 2)
)
