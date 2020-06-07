# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4

## execution time

|     benchmark     |     ruby      |     ruruby     |  rate  |
| :---------------: | :-----------: | :------------: | :----: |
| so_mandelbrot.rb  | 1.40 ± 0.07 s | 2.69 ± 0.16 s  | x 1.92 |
| app_mandelbrot.rb | 1.98 ± 0.01 s | 8.75 ± 0.21 s  | x 4.43 |
|      fibo.rb      | 0.50 ± 0.01 s | 2.59 ± 0.03 s  | x 5.15 |
|     block.rb      | 0.45 ± 0.01 s | 0.93 ± 0.01 s  | x 2.08 |
|    ao_bench.rb    | 7.84 ± 0.04 s | 34.63 ± 0.13 s | x 4.42 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate  |
| :---------------: | :---: | :----: | :----: |
| so_mandelbrot.rb  | 11.5K |  2.5K  | x 0.22 |
| app_mandelbrot.rb | 11.4K |  5.4K  | x 0.48 |
|      fibo.rb      | 11.1K |  2.1K  | x 0.19 |
|     block.rb      | 11.3K |  2.0K  | x 0.18 |
|    ao_bench.rb    | 11.6K | 11.4K  | x 0.98 |
