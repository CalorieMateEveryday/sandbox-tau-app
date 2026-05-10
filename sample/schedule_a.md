gantt
    dateFormat YYYY-MM-DD
    axisFormat %m/%d
    excludes weekends

    section dev1
    task1    :task1, 2026-05-10, 7d
    task2    :t2, 2026-05-13, 30d
    dev1m1    :milestone, m1, 2026-06-29, 7d  %% after:task1
    dev1m2    :milestone, 2026-09-1, 7d
    Crit    :crit, 2027-02-1, 7d

    section dev2
    dev1-t1    :milestone, 2026-05-10, 1d

    section dev3
    dev3-m    :milestone, 2026-012-1, 1d
