# ruruby benchmark results

## environment

Ruby version: 2.6.3  
CPU: Intel(R) Core(TM) i9-9880H CPU @ 2.30GHz  
OS: Mac OS X 10.15.4

## execution time

|     benchmark     |     ruby      |     ruruby     |  rate  |
| :---------------: | :-----------: | :------------: | :----: |
| so_mandelbrot.rb  | 1.48 ± 0.08 s | 2.78 ± 0.16 s  | x 1.88 |
| app_mandelbrot.rb | 1.98 ± 0.01 s | 8.77 ± 0.24 s  | x 4.44 |
|      fibo.rb      | 0.49 ± 0.01 s | 2.85 ± 0.19 s  | x 5.86 |
|     block.rb      | 0.47 ± 0.02 s | 0.98 ± 0.02 s  | x 2.11 |
|    ao_bench.rb    | 8.12 ± 0.15 s | 36.02 ± 0.37 s | x 4.44 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate  |
| :---------------: | :---: | :----: | :----: |
| so_mandelbrot.rb  | 11.6K |  2.4K  | x 0.20 |
| app_mandelbrot.rb | 11.2K |  3.7K  | x 0.33 |
|      fibo.rb      | 11.3K |  2.1K  | x 0.18 |
|     block.rb      | 11.7K |  2.0K  | x 0.17 |
|    ao_bench.rb    | 11.8K |  4.8K  | x 0.40 |
