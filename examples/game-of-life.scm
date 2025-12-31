(define create-coord
  (lambda (x y)
    (list x y)))

(define get-x
  (lambda (coord)
    (car coord)))

(define get-y
  (lambda (coord)
    (car (cdr coord))))

(define equal-coords?
  (lambda (c1 c2)
    (and
      (= (get-x c1) (get-x c2))
      (= (get-y c1) (get-y c2)))))

(define coord-in-list-of-coords?
  (lambda (coord list-of-coords)
    (cond
      ((null? list-of-coords) #f)
      ((equal-coords? coord (car list-of-coords)) #t)
      (else (coord-in-list-of-coords? coord (cdr list-of-coords))))))

(define no-duplicate-coords
  (lambda (list-of-coords)
    (cond
      ((null? list-of-coords) (list))
      ((coord-in-list-of-coords? (car list-of-coords) (cdr list-of-coords)) (no-duplicate-coords (cdr list-of-coords)))
      (else (cons (car list-of-coords) (no-duplicate-coords (cdr list-of-coords)))))))

(define neighbours
  (lambda (coord)
    (let ((x (get-x coord)) (y (get-y coord)))
      (list
        (create-coord x (+ y 1))
        (create-coord x (- y 1))
        (create-coord (+ x 1) y)
        (create-coord (- x 1) y)
        (create-coord (+ x 1) (+ y 1))
        (create-coord (- x 1) (- y 1))
        (create-coord (+ x 1) (- y 1))
        (create-coord (- x 1) (+ y 1))))))

(define neighbours-in-generation
  (lambda (cell generation)
    (filter
      (lambda (neighbour) (coord-in-list-of-coords? neighbour generation))
      (neighbours cell))))

(define cell-lives-on
  (lambda (cell generation)
    (let ((neighbours (neighbours-in-generation cell generation)))
      (or
        (= (length neighbours) 2)
        (= (length neighbours) 3)))))

(define cell-is-born
  (lambda (coord generation)
    (let ((neighbours (neighbours-in-generation coord generation)))
      (= (length neighbours) 3))))

(define consider-newborn-cells
  (lambda (generation)
    (cond
      ((null? generation) (list))
      (else
        (filter
          (lambda (coord) (not (coord-in-list-of-coords? coord generation)))
          (no-duplicate-coords
            (append
              (neighbours (car generation))
              (consider-newborn-cells (cdr generation)))))))))

(define next-generation
  (lambda (generation)
    (append
      (filter
        (lambda (cell) (cell-lives-on cell generation))
        generation)
      (filter
        (lambda (coord) (cell-is-born coord generation))
        (consider-newborn-cells generation)))))

(define game
  (lambda (generation)
    (cond
      ((null? generation)
       (begin
         (display "done")
        ))
      (else
        (begin
          (display generation)

          (game (next-generation generation)))))))

(game '((0 1) (1 1)))
