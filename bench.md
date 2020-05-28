# ruruby benchmark results

## environment

Ruby version: 2.7.1  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|     benchmark     |     ruby      |     ruruby     |  rate  |
| :---------------: | :-----------: | :------------: | :----: |
| so_mandelbrot.rb  | 1.73 ± 0.02 s | 2.56 ± 0.01 s  | x 1.48 |
| app_mandelbrot.rb | 2.14 ± 0.01 s | 6.07 ± 0.02 s  | x 2.83 |
|      fibo.rb      | 0.46 ± 0.01 s | 2.18 ± 0.02 s  | x 4.76 |
|     block.rb      | 0.36 ± 0.00 s | 1.02 ± 0.04 s  | x 2.82 |
|    ao_bench.rb    | 9.31 ± 0.15 s | 26.43 ± 0.17 s | x 2.84 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate  |
| :---------------: | :---: | :----: | :----: |
| so_mandelbrot.rb  | 14.2K |  8.9K  | x 0.63 |
| app_mandelbrot.rb | 14.2K | 26.2K  | x 1.84 |
|      fibo.rb      | 14.0K |  4.7K  | x 0.33 |
|     block.rb      | 14.1K |  4.6K  | x 0.33 |
|    ao_bench.rb    | 14.7K | 47.3K  | x 3.21 |
