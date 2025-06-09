(begin 
  (define a (map (lambda (x) (+ x 1)) (1 2 3)))

  (define fib (lambda (y) (
    if (< y 2) 0 a
  )))

  (fib ((+ 1 4)))
)
