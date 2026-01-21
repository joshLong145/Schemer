;;; Schemer Prelude - Minimal Standard Library
;;; Loaded implicitly before user code

;; List operations
(define map (lambda (f lst)
  (if (null? lst)
      '()
      (cons (f (car lst)) (map f (cdr lst))))))

(define filter (lambda (pred lst)
  (if (null? lst)
      '()
      (if (pred (car lst))
          (cons (car lst) (filter pred (cdr lst)))
          (filter pred (cdr lst))))))

(define length (lambda (lst)
  (if (null? lst) 0 (+ 1 (length (cdr lst))))))

(define append (lambda (lst1 lst2)
  (if (null? lst1) lst2 (cons (car lst1) (append (cdr lst1) lst2)))))

(define reverse (lambda (lst)
  (define rev-iter (lambda (lst acc)
    (if (null? lst) acc (rev-iter (cdr lst) (cons (car lst) acc)))))
  (rev-iter lst '())))

;; Folds
(define fold-right (lambda (f init lst)
  (if (null? lst) init (f (car lst) (fold-right f init (cdr lst))))))

(define fold-left (lambda (f init lst)
  (if (null? lst) init (fold-left f (f init (car lst)) (cdr lst)))))

;; Predicates
(define zero? (lambda (n) (= n 0)))
(define positive? (lambda (n) (> n 0)))
(define negative? (lambda (n) (< n 0)))
(define even? (lambda (n) (= (modulo n 2) 0)))
(define odd? (lambda (n) (not (even? n))))

;; List accessors
(define caar (lambda (x) (car (car x))))
(define cadr (lambda (x) (car (cdr x))))
(define cdar (lambda (x) (cdr (car x))))
(define cddr (lambda (x) (cdr (cdr x))))
