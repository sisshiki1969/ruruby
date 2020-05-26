# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4

## execution time

|     benchmark     |  ruby  | ruruby  |  rate  |
| :---------------: | :----: | :-----: | :----: |
| so_mandelbrot.rb  | 1.41 s | 3.29 s  | x 2.33 |
| app_mandelbrot.rb | 2.0 s  | 8.37 s  | x 4.18 |
|      fibo.rb      | 0.54 s |  2.7 s  | x 5.00 |
|     block.rb      | 0.44 s | 0.99 s  | x 2.25 |
|    ao_bench.rb    | 8.21 s | 33.88 s | x 4.13 |

## memory consumption

|     benchmark     |  ruby   | ruruby |   rate   |
| :---------------: | :-----: | :----: | :------: |
| so_mandelbrot.rb  | 11.40 K | 8.62 K |  x 0.76  |
| app_mandelbrot.rb | 0.01 M  | 1.78 M | x 146.76 |
|      fibo.rb      | 11.34 K | 2.06 K |  x 0.18  |
|     block.rb      | 11.38 K | 2.03 K |  x 0.18  |
|    ao_bench.rb    | 0.01 M  | 4.55 M | x 383.72 |
