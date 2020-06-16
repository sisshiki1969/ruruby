# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz  
OS: Ubuntu 18.04.4 LTS

## execution time

|     benchmark     |       ruby       |     ruruby      |  rate  |
| :---------------: | :--------------: | :-------------: | :----: |
| so_mandelbrot.rb  |  1.75 ± 0.01 s   |  2.85 ± 0.01 s  | x 1.63 |
| app_mandelbrot.rb |  2.20 ± 0.02 s   |  6.56 ± 0.07 s  | x 2.98 |
|      fibo.rb      |  0.47 ± 0.01 s   |  2.15 ± 0.01 s  | x 4.59 |
|     block.rb      |  0.40 ± 0.01 s   |  0.96 ± 0.01 s  | x 2.39 |
|    ao_bench.rb    |  8.95 ± 0.19 s   | 27.59 ± 0.16 s  | x 3.08 |
|     optcarrot     | 50.39 ± 0.61 fps | 8.32 ± 0.16 fps | x 6.06 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate  |
| :---------------: | :---: | :----: | :----: |
| so_mandelbrot.rb  | 13.8K |  4.8K  | x 0.35 |
| app_mandelbrot.rb | 13.9K | 17.4K  | x 1.25 |
|      fibo.rb      | 13.6K |  4.6K  | x 0.34 |
|     block.rb      | 13.6K |  4.6K  | x 0.34 |
|    ao_bench.rb    | 14.4K | 18.7K  | x 1.30 |
|     optcarrot     | 70.2K | 91.8K  | x 1.31 |
