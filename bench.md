# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz  
OS: Ubuntu 18.04.4 LTS

## execution time

|     benchmark     |     ruby      |     ruruby     |  rate  |
| :---------------: | :-----------: | :------------: | :----: |
| so_mandelbrot.rb  | 1.79 ± 0.02 s | 2.83 ± 0.06 s  | x 1.58 |
| app_mandelbrot.rb | 2.23 ± 0.02 s | 6.81 ± 0.08 s  | x 3.05 |
|      fibo.rb      | 0.47 ± 0.01 s | 2.45 ± 0.03 s  | x 5.26 |
|     block.rb      | 0.40 ± 0.00 s | 1.06 ± 0.01 s  | x 2.66 |
|    ao_bench.rb    | 9.09 ± 0.31 s | 29.49 ± 0.15 s | x 3.24 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate  |
| :---------------: | :---: | :----: | :----: |
| so_mandelbrot.rb  | 13.8K |  8.9K  | x 0.65 |
| app_mandelbrot.rb | 13.9K | 26.0K  | x 1.86 |
|      fibo.rb      | 13.6K |  4.6K  | x 0.34 |
|     block.rb      | 13.6K |  4.6K  | x 0.34 |
|    ao_bench.rb    | 14.4K | 47.4K  | x 3.29 |
