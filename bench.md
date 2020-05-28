# ruruby benchmark results

## environment

Ruby version: 2.7.1  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|     benchmark     |     ruby      |     ruruby     |  rate  |
| :---------------: | :-----------: | :------------: | :----: |
| so_mandelbrot.rb  | 1.72 ± 0.02 s | 2.66 ± 0.15 s  | x 1.55 |
| app_mandelbrot.rb | 2.16 ± 0.01 s | 7.05 ± 0.04 s  | x 3.27 |
|      fibo.rb      | 0.47 ± 0.02 s | 2.18 ± 0.05 s  | x 4.61 |
|     block.rb      | 0.37 ± 0.01 s | 1.00 ± 0.01 s  | x 2.71 |
|    ao_bench.rb    | 9.27 ± 0.05 s | 28.69 ± 1.62 s | x 3.10 |

## memory consumption

|     benchmark     | ruby  | ruruby |   rate   |
| :---------------: | :---: | :----: | :------: |
| so_mandelbrot.rb  | 14.2K | 11.7K  |  x 0.82  |
| app_mandelbrot.rb | 0.0M  |  1.8M  | x 123.78 |
|      fibo.rb      | 14.1K |  4.6K  |  x 0.33  |
|     block.rb      | 14.1K |  4.6K  |  x 0.33  |
|    ao_bench.rb    | 0.0M  |  4.5M  | x 306.41 |
