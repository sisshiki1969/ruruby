# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz  
OS: Ubuntu 18.04.4 LTS

## execution time

|     benchmark     |     ruby      |     ruruby     |  rate  |
| :---------------: | :-----------: | :------------: | :----: |
| so_mandelbrot.rb  | 1.70 ± 0.03 s | 2.70 ± 0.05 s  | x 1.59 |
| app_mandelbrot.rb | 2.20 ± 0.04 s | 6.71 ± 0.04 s  | x 3.06 |
|      fibo.rb      | 0.46 ± 0.00 s | 2.14 ± 0.01 s  | x 4.70 |
|     block.rb      | 0.43 ± 0.01 s | 1.00 ± 0.01 s  | x 2.34 |
|    ao_bench.rb    | 8.95 ± 0.29 s | 29.02 ± 0.73 s | x 3.24 |

## optcarrot benchmark

|    benchmark    |       ruby        |      ruruby      |  rate  |
| :-------------: | :---------------: | :--------------: | :----: |
|    optcarrot    | 49.99 ± 0.43 fps  | 7.82 ± 0.21 fps  | x 6.39 |
| optcarrot --opt | 142.43 ± 2.73 fps | 35.96 ± 1.24 fps | x 3.96 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate   |
| :---------------: | :---: | :----: | :-----: |
| so_mandelbrot.rb  | 13.8K |  4.8K  | x 0.34  |
| app_mandelbrot.rb | 13.9K | 17.3K  | x 1.25  |
|      fibo.rb      | 13.6K |  4.6K  | x 0.34  |
|     block.rb      | 13.6K |  4.6K  | x 0.34  |
|    ao_bench.rb    | 14.3K | 20.8K  | x 1.45  |
|     optcarrot     | 70.3K | 91.9K  | x 1.31  |
|  optcarrot --opt  | 0.1M  |  4.3M  | x 48.98 |
