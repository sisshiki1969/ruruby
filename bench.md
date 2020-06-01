# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz  
OS: Ubuntu 18.04.4 LTS

## execution time

|     benchmark     |     ruby      |     ruruby     |  rate  |
| :---------------: | :-----------: | :------------: | :----: |
| so_mandelbrot.rb  | 1.78 ± 0.02 s | 3.28 ± 0.13 s  | x 1.85 |
| app_mandelbrot.rb | 2.36 ± 0.10 s | 6.67 ± 0.08 s  | x 2.82 |
|      fibo.rb      | 0.46 ± 0.01 s | 2.52 ± 0.03 s  | x 5.46 |
|     block.rb      | 0.41 ± 0.00 s | 1.05 ± 0.01 s  | x 2.60 |
|    ao_bench.rb    | 9.21 ± 0.04 s | 28.91 ± 0.45 s | x 3.14 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate  |
| :---------------: | :---: | :----: | :----: |
| so_mandelbrot.rb  | 13.9K |  4.7K  | x 0.34 |
| app_mandelbrot.rb | 13.9K |  4.9K  | x 0.35 |
|      fibo.rb      | 13.6K |  4.6K  | x 0.34 |
|     block.rb      | 13.6K |  4.5K  | x 0.33 |
|    ao_bench.rb    | 14.3K |  8.6K  | x 0.60 |
