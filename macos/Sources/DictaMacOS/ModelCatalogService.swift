import CryptoKit
import Darwin
import Foundation

struct CatalogModel: Equatable, Sendable {
    let id: String
    let name: String
    let sizeBytes: UInt64
    let sha256: String
    let description: String
    let filename: String

    var downloadURL: URL {
        URL(string: "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/\(filename)")!
    }
}

struct ModelCatalogService {
    static let revision = "5359861c739e955e79d9a303bcbc70fb988958b1"
    static let models = [
        CatalogModel(id: "tiny", name: "Whisper Tiny", sizeBytes: 77_691_713,
                     sha256: "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
                     description: "Smallest multilingual model", filename: "ggml-tiny.bin"),
        CatalogModel(id: "base", name: "Whisper Base", sizeBytes: 147_951_465,
                     sha256: "60ed5bc3dd14eea856493d334349b405782ddcaf0028d4b5df4088345fba2efe",
                     description: "Balanced multilingual model", filename: "ggml-base.bin"),
        CatalogModel(id: "small", name: "Whisper Small", sizeBytes: 487_601_967,
                     sha256: "1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b",
                     description: "Higher quality multilingual model", filename: "ggml-small.bin")
    ]

    func listModels(in directory: URL) throws -> [ModelCatalogEntry] {
        Self.models.map { model in
            let path = directory.appendingPathComponent(model.filename)
            let exists = Self.pathExistsWithoutFollowingSymlinks(path)
            let installed: Bool
            if exists {
                installed = (try? Self.verify(path: path, model: model)) == true
            } else {
                installed = false
            }
            return ModelCatalogEntry(model: model, installed: installed)
        }
    }

    func delete(modelID: String, from directory: URL) throws {
        guard let model = Self.models.first(where: { $0.id == modelID }) else {
            throw ModelCatalogError.unknownModel(modelID)
        }
        try FileManager.default.removeItem(at: directory.appendingPathComponent(model.filename))
    }

    /// This is only called following an explicit user request. Cancelling the task cancels URLSession.
    func download(modelID: String, to directory: URL, session: URLSession = .shared) async throws -> URL {
        guard let model = Self.models.first(where: { $0.id == modelID }) else {
            throw ModelCatalogError.unknownModel(modelID)
        }
        try Task.checkCancellation()
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let root = directory.resolvingSymlinksInPath().standardizedFileURL
        let destination = root.appendingPathComponent(model.filename)
        guard !Self.pathExistsWithoutFollowingSymlinks(destination) else {
            throw ModelCatalogError.destinationExists
        }
        let temp = root.appendingPathComponent(".\(model.id).\(UUID().uuidString).download")
        let (downloadURL, response) = try await session.download(from: model.downloadURL)
        guard (response as? HTTPURLResponse)?.statusCode == 200 else { throw ModelCatalogError.badResponse }
        try Task.checkCancellation()
        do {
            try FileManager.default.moveItem(at: downloadURL, to: temp)
            _ = try Self.verify(path: temp, model: model)
            try Task.checkCancellation()
            // A hard link installs without replacing a file created concurrently by another process.
            guard link(temp.path, destination.path) == 0 else {
                if Self.pathExistsWithoutFollowingSymlinks(destination) { throw ModelCatalogError.destinationExists }
                throw ModelCatalogError.installFailed(String(cString: strerror(errno)))
            }
            try FileManager.default.removeItem(at: temp)
            return destination
        } catch {
            try? FileManager.default.removeItem(at: temp)
            throw error
        }
    }

    private static func pathExistsWithoutFollowingSymlinks(_ url: URL) -> Bool {
        var info = stat()
        return lstat(url.path, &info) == 0
    }

    static func verify(path: URL, model: CatalogModel) throws -> Bool {
        let values = try path.resourceValues(forKeys: [.isRegularFileKey, .fileSizeKey])
        guard values.isRegularFile == true else { throw ModelCatalogError.notRegularFile }
        guard let size = values.fileSize, UInt64(size) == model.sizeBytes else { throw ModelCatalogError.invalidSize }
        let handle = try FileHandle(forReadingFrom: path)
        defer { try? handle.close() }
        var hasher = SHA256()
        while let chunk = try handle.read(upToCount: 1024 * 1024), !chunk.isEmpty { hasher.update(data: chunk) }
        guard Data(hasher.finalize()).map({ String(format: "%02x", $0) }).joined() == model.sha256 else {
            throw ModelCatalogError.invalidChecksum
        }
        return true
    }
}

struct ModelCatalogEntry: Sendable {
    let model: CatalogModel
    let installed: Bool
}

enum ModelCatalogError: Error, LocalizedError {
    case unknownModel(String), destinationExists, badResponse, notRegularFile, invalidSize, invalidChecksum, installFailed(String)
    var errorDescription: String? {
        switch self {
        case let .unknownModel(id): "Unknown model: \(id)"
        case .destinationExists: "Model destination already exists; existing files are never replaced."
        case .badResponse: "Model server returned an unsuccessful response."
        case .notRegularFile: "Model is not a regular file."
        case .invalidSize: "Model size does not match the catalog."
        case .invalidChecksum: "Model SHA-256 does not match the catalog."
        case let .installFailed(message): "Could not install model: \(message)"
        }
    }
}
