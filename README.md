# Requirement Manager (ReqMan)

This software is a basic management for requirements and tests. 

## ToDo List
+ [X] Hierarchy for
  + [X] Requirements
  + [X] Tests
+ [ ] Operations logging
+ [ ] Parsers for requirements
  + [ ] Latex files (Write a command)
  + [ ] Word files (Write a macro)
  + [ ] Excel files
+ [ ] Parsers for tests
  + [ ] Doxygen documentation
  + [ ] ...
+ [ ] Better webpage 
  + [X] Use templates (based on hbs)
+ [ ] Reports generator
  + [ ] Latex template
  + [X] Excel 
  + [ ] PDF document
+ [ ] Multiples projects
+ [x] REST API (partially)
+ [ ] Optimize DB access
  + [ ] Reduce SQL queries
  + [ ] DB pool
+ [ ] Security
  + [ ] Use https
  + [ ] users/admin
+ Snapshots
  + [ ] Configuration management
+ Better error management (remove all unwrap())

## API 

At http://localhost:8000/api there is a REST API with the following endpoints:

+ api/requirements        (GET/POST)
+ api/requirements/\<id\> (GET)
+ api/tests               (GET)
+ api/tests/\<id\>        (GET)
+ api/status              (GET)
+ api/categories          (GET)
+ api/matrix              (GET)

## JSON format

TBD

# Database

This prototype is based on Postgres. See docker container for set-up the database.

## Schema

See entity diagram
![](doc/ER%20diagram.png)

# Running

Terminal 1

```
> docker-compose run
```

Terminal 2

```
> diesel setup
> diesel migration redo
> cargo run
```

It should publish a webpage at http://localhost:8000

