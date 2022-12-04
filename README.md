# Requirement Manager (ReqMan)

This software is a basic management for requirements and tests. 

## ToDo List
+ [ ] Requirements hierarchy 
+ [ ] Operations history
+ [ ] Parsers for requirements
  + [ ] Latex files (Write a command)
  + [ ] Word files (Write a macro)
+ [ ] Parser for tests
  + [ ] Doxygen documentation
+ [ ] Better webpage 
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


## Installation

```
> diesel setup
> diesel migration redo
> cargo run
```

It should publish a webpage at http://localhost:8080 


## API 

At http://localhost:8080/api there is a REST API with the following endpoints:

+ api/requirements  (GET/POST)
+ api/requirements/\<id\> (GET)
+ api/status        (GET)
+ api/categories    (GET)
+ api/matrixID      (GET)

## JSON format




# Database

This prototype is based on Postgres. See docker container for set-up the database.

## Running

```
> docker-compose run
```

## Schema

See entity diagram
![](doc/entity.png)
