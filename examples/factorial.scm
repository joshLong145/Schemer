(begin
    (define factorial (lambda (n)
        (if (< n 2)
            1
            (* n (factorial (- n 1)))
        ))
    )

    (display (factorial 5))
)
