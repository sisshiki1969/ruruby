# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4

## execution time

|     benchmark     |  ruby  | ruruby |  rate  |
| :---------------: | :----: | :----: | :----: |
| so_mandelbrot.rb  | 1.32 s | 2.84 s | x 2.15 |
| app_mandelbrot.rb | 2.06 s | 8.07 s | x 3.92 |
|      fibo.rb      | 0.51 s | 2.58 s | x 5.06 |
|     block.rb      | 0.43 s | 0.88 s | x 2.05 |
|    ao_bench.rb    | 7.56 s | 33.0 s | x 4.37 |

## memory consumption

|     benchmark     |  ruby   | ruruby |   rate   |
| :---------------: | :-----: | :----: | :------: |
| so_mandelbrot.rb  | 11.49 K | 5.78 K |  x 0.50  |
| app_mandelbrot.rb | 0.01 M  | 1.78 M | x 151.41 |
|      fibo.rb      | 11.20 K | 2.08 K |  x 0.19  |
|     block.rb      | 11.39 K | 2.04 K |  x 0.18  |
|    ao_bench.rb    | 0.01 M  | 4.55 M | x 396.48 |

- /usr/bin/time ruby ../optcarrot/bin/optcarrot-bench
  fps: 44.32798770847959
  4.39 real 4.33 user 0.03 sys
- /usr/bin/time target/release/ruruby ../optcarrot/bin/optcarrot-bench
  fps: 9.716091435998994
  19.57 real 19.49 user 0.07 sys
