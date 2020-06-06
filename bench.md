# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz  
OS: Ubuntu 18.04.4 LTS

## execution time

|     benchmark     |     ruby      |     ruruby     |  rate  |
| :---------------: | :-----------: | :------------: | :----: |
| so_mandelbrot.rb  | 1.83 ± 0.02 s | 2.83 ± 0.02 s  | x 1.55 |
| app_mandelbrot.rb | 2.23 ± 0.03 s | 7.33 ± 0.17 s  | x 3.29 |
|      fibo.rb      | 0.51 ± 0.06 s | 2.44 ± 0.02 s  | x 4.81 |
|     block.rb      | 0.42 ± 0.00 s | 1.12 ± 0.02 s  | x 2.69 |
|    ao_bench.rb    | 9.50 ± 0.36 s | 29.56 ± 0.97 s | x 3.11 |

## memory consumption

|     benchmark     | ruby  | ruruby |   rate   |
| :---------------: | :---: | :----: | :------: |
| so_mandelbrot.rb  | 13.8K |  9.0K  |  x 0.65  |
| app_mandelbrot.rb | 0.0M  |  1.8M  | x 128.36 |
|      fibo.rb      | 13.6K |  4.5K  |  x 0.33  |
|     block.rb      | 13.5K |  4.6K  |  x 0.34  |
|    ao_bench.rb    | 0.0M  |  4.5M  | x 315.56 |
