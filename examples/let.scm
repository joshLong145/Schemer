(begin
    (define foo (1 2 3))
    (define adder (lambda (x y) (let
            (
                (define a (car foo))
                (define b (car (cdr foo)))
            )
            (list a b x y)
        )
    ))

    (adder (1 2))
)
