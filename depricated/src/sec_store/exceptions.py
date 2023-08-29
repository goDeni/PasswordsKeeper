class WrongKeyHashError(Exception):
    pass


class RepositoryError(Exception):
    pass


class RecordAlreadyExist(RepositoryError):
    pass
