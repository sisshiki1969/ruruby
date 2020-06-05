# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4

## execution time

|     benchmark     |     ruby      |     ruruby     |  rate  |
| :---------------: | :-----------: | :------------: | :----: |
| so_mandelbrot.rb  | 1.32 ± 0.04 s | 2.48 ± 0.04 s  | x 1.88 |
| app_mandelbrot.rb | 2.03 ± 0.12 s | 8.56 ± 0.07 s  | x 4.22 |
|      fibo.rb      | 0.50 ± 0.01 s | 2.72 ± 0.03 s  | x 5.42 |
|     block.rb      | 0.44 ± 0.00 s | 1.00 ± 0.06 s  | x 2.28 |
|    ao_bench.rb    | 7.74 ± 0.06 s | 33.70 ± 0.55 s | x 4.35 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate  |
| :---------------: | :---: | :----: | :----: |
| so_mandelbrot.rb  | 11.2K |  2.3K  | x 0.21 |
| app_mandelbrot.rb | 11.5K |  3.7K  | x 0.33 |
|      fibo.rb      | 11.3K |  2.1K  | x 0.18 |
|     block.rb      | 11.4K |  2.0K  | x 0.18 |
|    ao_bench.rb    | 11.8K |  4.5K  | x 0.38 |
