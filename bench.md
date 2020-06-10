# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz  
OS: Ubuntu 18.04.4 LTS

## execution time

|     benchmark     |     ruby      |     ruruby     |  rate  |
| :---------------: | :-----------: | :------------: | :----: |
| so_mandelbrot.rb  | 1.80 ± 0.04 s | 2.99 ± 0.03 s  | x 1.66 |
| app_mandelbrot.rb | 2.26 ± 0.03 s | 6.65 ± 0.03 s  | x 2.94 |
|      fibo.rb      | 0.47 ± 0.02 s | 2.47 ± 0.03 s  | x 5.25 |
|     block.rb      | 0.41 ± 0.00 s | 1.03 ± 0.03 s  | x 2.49 |
|    ao_bench.rb    | 9.40 ± 0.09 s | 29.74 ± 0.96 s | x 3.16 |

## memory consumption

|     benchmark     | ruby  | ruruby |   rate   |
| :---------------: | :---: | :----: | :------: |
| so_mandelbrot.rb  | 13.9K |  4.9K  |  x 0.35  |
| app_mandelbrot.rb | 13.8K |  6.1K  |  x 0.44  |
|      fibo.rb      | 13.6K |  4.7K  |  x 0.34  |
|     block.rb      | 13.6K |  4.6K  |  x 0.34  |
|    ao_bench.rb    | 0.0M  |  4.5M  | x 315.55 |
